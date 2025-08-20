use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub id: Option<u64>,
    pub result: Option<Value>,
    pub error: Option<Value>,
}
