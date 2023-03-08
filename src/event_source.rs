use futures::stream::StreamExt;
use futures::{stream, Stream};

use ergo_chain_sync::ChainUpgrade;
use ergo_mempool_sync::{Combined, MempoolUpdate};
use futures::future::ready;

use crate::models::kafka_event::BlockEvent;
use crate::models::mempool_event::MempoolEvent;
use kafka::producer::{Producer, Record};

use crate::models::tx_event::TxEvent;

pub fn block_event_source<S>(
    upstream: S,
    mut producer: Producer,
    topic: String,
) -> impl Stream<Item = ChainUpgrade>
where
    S: Stream<Item = ChainUpgrade>,
{
    upstream.then(move |ev| {
        let block_event = BlockEvent::from_chain_upgrade(ev.clone());
        let block_id: String = match block_event.clone() {
            BlockEvent::BlockApply {
                timestamp: _,
                height: _,
                id,
            } => id.clone(),
            BlockEvent::BlockUnapply {
                timestamp: _,
                height: _,
                id,
            } => id.clone(),
        };
        let value = serde_json::to_string(&block_event).unwrap();
        println!("Block value is: ${:?}", value.clone());
        let rec: &Record<String, String> =
            &Record::from_key_value(topic.as_str(), block_id.clone(), value);
        println!("Got new block. Key: ${:?}", block_id.clone());
        producer.send(rec).unwrap();
        println!("New block processed by kafka. Key: ${:?}", block_id.clone());
        ready(ev.clone())
    })
}

pub fn tx_event_source<S>(upstream: S) -> impl Stream<Item = TxEvent>
where
    S: Stream<Item = ChainUpgrade>,
{
    upstream.flat_map(|u| stream::iter(process_upgrade(u)))
}

fn process_upgrade(upgr: ChainUpgrade) -> Vec<TxEvent> {
    match upgr {
        ChainUpgrade::RollForward(blk) => blk
            .transactions
            .into_iter()
            .map(|tx| TxEvent::AppliedTx {
                tx,
                timestamp: blk.timestamp as i64,
                block_height: blk.height as i32,
            })
            .collect(),
        ChainUpgrade::RollBackward(blk) => {
            blk.transactions
                .into_iter()
                .rev() // we unapply txs in reverse order.
                .map(TxEvent::UnappliedTx)
                .collect()
        }
    }
}

pub fn mempool_event_source<S>(
    upstream: S,
    mut producer: Producer,
    topic: String,
) -> impl Stream<Item = ()>
where
    S: Stream<Item = Combined>,
{
    upstream
        .map(|ev| match ev {
            Combined::Mempool(mempool) => Some(mempool),
            _ => None,
        })
        .then(move |event| {
            match event {
                Some(ev) => {
                    let kafka_event = MempoolEvent::from_mempool_event(ev.clone());
                    if kafka_event.is_some() {
                        let kafka_string = serde_json::to_string(&kafka_event.unwrap()).unwrap();
                        let tx_id: String = match ev.clone() {
                            MempoolUpdate::TxAccepted(tx) => tx.id().clone().into(),
                            MempoolUpdate::TxWithdrawn(tx) => tx.id().clone().into(),
                            _ => "".to_string(),
                        };
                        let rec: &Record<String, String> =
                            &Record::from_key_value(topic.as_str(), tx_id.clone(), kafka_string);
                        println!("Got new mempool event. Key: ${:?}", tx_id.clone());
                        producer.send(rec).unwrap();
                        println!(
                            "New mempool event processed by kafka. Key: ${:?}",
                            tx_id.clone()
                        );
                    }
                }
                _ => {}
            }
            ready(())
        })
}
