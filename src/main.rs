mod handlers;
mod models;
mod event_source;

use std::sync::{Once};
use futures::{Stream};
use std::pin::Pin;
use isahc::{prelude::*, HttpClient};
use ergo_chain_sync::{chain_sync_stream, ChainSync, ChainSyncNonInit};
use ergo_chain_sync::client::node::ErgoNodeHttpClient;
use ergo_chain_sync::cache::rocksdb::ChainCacheRocksDB;
use serde::Deserialize;
use clap::{arg, Parser};
use ergo_chain_sync::rocksdb::RocksConfig;
use ergo_chain_sync::client::types::Url;
use std::time::Duration;
use ergo_mempool_sync::{mempool_sync_stream_combined, MempoolSync, MempoolSyncConf};

use kafka::producer::{Producer, RequiredAcks};
use futures::StreamExt;

use crate::handlers::proxy::ProxyEvents;
use spectrum_offchain::event_sink::types::{EventHandler, NoopDefaultHandler};

use spectrum_offchain::event_sink::process_events;
use futures::stream::select_all;
use crate::event_source::{block_event_source, mempool_event_source, tx_event_source};
use crate::models::tx_event::TxEvent;
use ergo_mempool_sync::client::node::ErgoMempoolHttpClient;

#[tokio::main]
async fn main() {
    let args = AppArgs::parse();
    let raw_config = std::fs::read_to_string(args.config_yaml_path).expect("Cannot load configuration file");
    let config: AppConfig = serde_yaml::from_str(&raw_config).expect("Invalid configuration file");

    if let Some(log4rs_path) = args.log4rs_path {
        log4rs::init_file(log4rs_path, Default::default()).unwrap();
    } else {
        log4rs::init_file(config.log4rs_yaml_path, Default::default()).unwrap();
    }
    let client = HttpClient::builder()
        .timeout(std::time::Duration::from_secs(
            config.http_client_timeout_duration_secs.clone() as u64,
        ))
        .build()
        .unwrap();

    let node = ErgoNodeHttpClient::new(client, config.node_addr.clone());
    let cache = ChainCacheRocksDB::new(RocksConfig {
        db_path: config.chain_cache_db_path.into(),
    });
    static SIGNAL_TIP_REACHED: Once = Once::new();
    let chain_sync = ChainSync::init(
        config.chain_sync_starting_height.clone(),
        &node,
        cache,
        Some(&SIGNAL_TIP_REACHED),
    ).await;
    let client_mempool = HttpClient::builder()
        .timeout(std::time::Duration::from_secs(
            config.http_client_timeout_duration_secs.clone() as u64,
        ))
        .build()
        .unwrap();
    let node_mempool = ErgoMempoolHttpClient::new(client_mempool, config.node_addr.clone());
    let cache_mempool = ChainCacheRocksDB::new(RocksConfig {
        db_path: config.mempool_cache_db_path.into(),
    });


    let producer1 =
        Producer::from_hosts(vec!(config.kafka_address.to_owned()))
            .with_ack_timeout(Duration::from_secs(1))
            .with_required_acks(RequiredAcks::One)
            .create()
            .unwrap();
    let producer2 =
        Producer::from_hosts(vec!(config.kafka_address.to_owned()))
            .with_ack_timeout(Duration::from_secs(1))
            .with_required_acks(RequiredAcks::One)
            .create()
            .unwrap();
    let producer3 =
        Producer::from_hosts(vec!(config.kafka_address.to_owned()))
            .with_ack_timeout(Duration::from_secs(1))
            .with_required_acks(RequiredAcks::One)
            .create()
            .unwrap();

    let mempool_chain_sync =
        ChainSyncNonInit::new(
            &node,
            cache_mempool,
        );


    let mempool_sync = MempoolSync::init(
        MempoolSyncConf {
            sync_interval: config.mempool_sync_interval
        },
        &node_mempool,
        mempool_chain_sync,
    ).await;

    let mempool_source = mempool_event_source(
        mempool_sync_stream_combined(mempool_sync),
        producer3, config.mempool_topic.to_string(),
    );
    let event_source = tx_event_source(
        block_event_source(chain_sync_stream(chain_sync), producer1, config.blocks_topic.to_string())
    );
    let handler = ProxyEvents::new(producer2, config.tx_topic.clone().to_string());
    let handlers: Vec<Box<dyn EventHandler<TxEvent>>> = vec![
        Box::new(handler)
    ];


    let default_handler = NoopDefaultHandler;
    let process_events_stream = boxed(
        process_events(event_source, handlers, default_handler)
    );

    let mut app = select_all(vec![
        process_events_stream,
        boxed(mempool_source),
    ]);

    loop {
        app.select_next_some().await;
    }
}

#[derive(Deserialize)]
struct AppConfig<'a> {
    node_addr: Url,
    http_client_timeout_duration_secs: u32,
    chain_sync_starting_height: u32,
    log4rs_yaml_path: &'a str,
    chain_cache_db_path: &'a str,
    mempool_cache_db_path: &'a str,
    kafka_address: &'a str,
    blocks_topic: &'a str,
    tx_topic: &'a str,
    mempool_topic: &'a str,
    mempool_sync_interval: u64,
}

#[derive(Parser)]
#[command(name = "events-streaming")]
#[command(version = "0.1")]
#[command(about = "", long_about = None)]
struct AppArgs {
    /// Path to the YAML configuration file.
    #[arg(long, short)]
    config_yaml_path: String,
    /// Optional path to the log4rs YAML configuration file. NOTE: overrides path specified in config YAML file.
    #[arg(long, short)]
    log4rs_path: Option<String>,
}

pub fn boxed<'a, T>(s: impl Stream<Item=T> + 'a) -> Pin<Box<dyn Stream<Item=T> + 'a>> {
    Box::pin(s)
}