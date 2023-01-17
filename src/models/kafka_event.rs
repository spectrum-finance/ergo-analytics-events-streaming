use ergo_lib::ergotree_ir::serialization::SigmaSerializable;

use base64::{Engine as _, engine::general_purpose};
use ergo_chain_sync::ChainUpgrade;
use ergo_chain_sync::model::Block;

use serde::{Deserialize, Serialize};
use crate::models::tx_event::TxEvent;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum KafkaEvent {
    AppliedEvent { timestamp: i64, tx: String, height: i32 },
    UnappliedEvent(String),
}

impl KafkaEvent {
    pub fn from_tx_ledger_event(ev: TxEvent) -> Self {
        match ev {
            TxEvent::AppliedTx { timestamp, tx, block_height } => {
                let tx_bytes: Vec<u8> = tx.sigma_serialize_bytes().unwrap();
                let encoded: String = general_purpose::STANDARD_NO_PAD.encode(tx_bytes);
                KafkaEvent::AppliedEvent {
                    timestamp,
                    tx: encoded,
                    height: block_height,
                }
            }
            TxEvent::UnappliedTx(tx) => {
                let tx_bytes: Vec<u8> = tx.sigma_serialize_bytes().unwrap();
                let encoded: String = general_purpose::STANDARD_NO_PAD.encode(tx_bytes);
                KafkaEvent::UnappliedEvent(encoded)
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum BlockEvent {
    BlockApply { timestamp: u64, height: u32, id: String },
    BlockUnapply { timestamp: u64, height: u32, id: String },
}

impl BlockEvent {
    pub fn from_chain_upgrade(ev: ChainUpgrade) -> Self {
        match ev {
            ChainUpgrade::RollForward(Block { id, parent_id: _, height, timestamp, transactions: _ }) => {
                let id: String = base16::encode_lower(id.0.0.as_ref());
                BlockEvent::BlockApply { timestamp, height, id }
            }
            ChainUpgrade::RollBackward(Block { id, parent_id: _, height, timestamp, transactions: _ }) => {
                let id: String = base16::encode_lower(id.0.0.as_ref());
                BlockEvent::BlockUnapply { timestamp, height, id }
            }
        }
    }
}