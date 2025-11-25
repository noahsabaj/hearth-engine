//! Atomic Save Data - Stub
use super::{SaveOperation, SavePriority};

pub struct AtomicSaveData;

impl AtomicSaveData {
    pub fn queue_operation(&self, _op: SaveOperation) -> crate::persistence::PersistenceResult<()> {
        // Stub implementation
        Ok(())
    }
}
