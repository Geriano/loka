use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    user: String,
    worker: String,
}

impl State {
    pub fn new(user: &str, worker: &str) -> Self {
        Self {
            user: user.to_owned(),
            worker: worker.to_owned(),
        }
    }

    pub fn user(&self) -> &str {
        &self.user
    }

    pub fn worker(&self) -> &str {
        &self.worker
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.user, self.worker)
    }
}
