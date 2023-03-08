use spectrum_offchain::event_sink::types::EventHandler;

use kafka::producer::{Producer, Record};

use async_trait::async_trait;

use crate::models::kafka_event::KafkaEvent;
use crate::models::tx_event::TxEvent;

pub struct ProxyEvents {
    pub producer: Producer,
    pub topic: String,
}

impl ProxyEvents {
    pub fn new(producer: Producer, topic: String) -> Self {
        Self { producer, topic }
    }
}

#[async_trait(? Send)]
impl EventHandler<TxEvent> for ProxyEvents {
    async fn try_handle(&mut self, ev: TxEvent) -> Option<TxEvent> {
        let kafka_event = KafkaEvent::from_tx_ledger_event(ev.clone());
        let kafka_string = serde_json::to_string(&kafka_event).unwrap();
        let tx_id: String = match ev.clone() {
            TxEvent::AppliedTx {
                timestamp: _timestamp,
                tx,
                block_height: _,
            } => tx.id().clone().into(),
            TxEvent::UnappliedTx(tx) => tx.id().clone().into(),
        };
        let rec: &Record<String, String> =
            &Record::from_key_value(self.topic.as_str(), tx_id.clone(), kafka_string);
        println!("Got new event. Key: ${:?}", tx_id.clone());
        self.producer.send(rec).unwrap();
        println!("New event processed by kafka. Key: ${:?}", tx_id.clone());
        Some(ev.clone())
    }
}
