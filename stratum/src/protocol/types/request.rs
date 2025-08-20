use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::method::Method;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: Option<u64>,
    pub method: Method,
    pub params: Option<Value>,
}
