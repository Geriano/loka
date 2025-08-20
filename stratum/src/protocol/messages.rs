use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StratumMessage {
    Subscribe {
        id: u64,
        user_agent: Option<String>,
    },
    Authenticate {
        id: u64,
        user: String,
        worker: String,
        password: Option<String>,
    },
    SetDifficulty {
        difficulty: f64,
    },
    Notify {
        job_id: String,
    },
    Submit {
        id: u64,
        job_id: String,
    },
    Submitted {
        id: u64,
        valid: bool,
    },
}

impl StratumMessage {
    pub fn message_type(&self) -> &'static str {
        match self {
            Self::Subscribe { .. } => "subscribe",
            Self::Authenticate { .. } => "authenticate",
            Self::SetDifficulty { .. } => "set_difficulty",
            Self::Notify { .. } => "notify",
            Self::Submit { .. } => "submit",
            Self::Submitted { .. } => "submitted",
        }
    }
    
    pub fn id(&self) -> Option<u64> {
        match self {
            Self::Subscribe { id, .. } => Some(*id),
            Self::Authenticate { id, .. } => Some(*id),
            Self::Submit { id, .. } => Some(*id),
            Self::Submitted { id, .. } => Some(*id),
            _ => None,
        }
    }
}

