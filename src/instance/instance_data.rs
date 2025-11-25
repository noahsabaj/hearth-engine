//! Instance Data - Stub
use super::InstanceId;

pub struct InstanceData {
    pub ids: Vec<InstanceId>,
    pub types: Vec<InstanceType>,
    pub active: Vec<bool>,
    pub created_at: Vec<u64>,
    pub created_by: Vec<InstanceId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum InstanceType { Static, Dynamic }

pub struct InstanceManagerData {
    pub ids: Vec<InstanceId>,
    pub types: Vec<InstanceType>,
    pub active: Vec<bool>,
    pub created_at: Vec<u64>,
    pub created_by: Vec<InstanceId>,
}
