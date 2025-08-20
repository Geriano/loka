#[derive(Debug, Clone)]
pub enum Message {
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
