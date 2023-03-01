use base64::Engine;
use base64::engine::general_purpose;
use ergo_lib::ergotree_ir::serialization::SigmaSerializable;
use ergo_mempool_sync::MempoolUpdate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum MempoolEvent {
    TxAccepted { tx: String },
    TxWithdrawn { tx: String },
}

impl MempoolEvent {
    pub fn from_mempool_event(ev: MempoolUpdate) -> Option<Self> {
        match ev {
            MempoolUpdate::TxAccepted(tx) => {
                let tx_bytes: Vec<u8> = tx.sigma_serialize_bytes().unwrap();
                let encoded: String = general_purpose::STANDARD_NO_PAD.encode(tx_bytes);
                Some(MempoolEvent::TxAccepted { tx: encoded })
            }
            MempoolUpdate::TxWithdrawn(tx) => {
                let tx_bytes: Vec<u8> = tx.sigma_serialize_bytes().unwrap();
                let encoded: String = general_purpose::STANDARD_NO_PAD.encode(tx_bytes);
                Some(MempoolEvent::TxWithdrawn { tx: encoded })
            }
            _ => {
                None
            }
        }
    }
}