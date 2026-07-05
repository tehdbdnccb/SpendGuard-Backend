use tokio::sync::broadcast;

use crate::domain::RequestLog;
use crate::infra::{BtlClient, Db};

const TELEMETRY_CHANNEL_CAPACITY: usize = 256;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub btl: BtlClient,
    pub telemetry_tx: broadcast::Sender<RequestLog>,
}

impl AppState {
    pub fn new(db: Db, btl: BtlClient) -> Self {
        let (telemetry_tx, _rx) = broadcast::channel(TELEMETRY_CHANNEL_CAPACITY);
        Self { db, btl, telemetry_tx }
    }

    pub fn publish_telemetry(&self, log: RequestLog) {
        let _ = self.telemetry_tx.send(log);
    }
}
