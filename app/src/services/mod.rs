use domain::core::BrokerX;
use std::sync::Arc;

/// Lightweight handle to the BrokerX system that can be cheaply cloned across threads
/// BrokerX already has internal thread safety through ProcessingPool's shared_state
#[derive(Clone)]
pub struct BrokerHandle {
    inner: Arc<BrokerX>,
}

impl BrokerHandle {
    pub fn new(broker: BrokerX) -> Self {
        Self {
            inner: Arc::new(broker),
        }
    }

    /// Get a reference to the broker - direct access since BrokerX handles internal sync
    pub fn broker(&self) -> &BrokerX {
        &self.inner
    }
}
