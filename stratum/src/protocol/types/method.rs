use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Method {
    #[serde(rename = "mining.subscribe")]
    Subscribe,
    #[serde(rename = "mining.authorize")]
    Authorize,
    #[serde(rename = "mining.submit")]
    Submit,
    #[serde(rename = "mining.notify")]
    Notify,
    #[serde(rename = "mining.set_difficulty")]
    SetDifficulty,
    #[serde(rename = "client.reconnect")]
    Reconnect,
    Other(String),
}
