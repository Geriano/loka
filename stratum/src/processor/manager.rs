use crate::processor::Message;
use crate::protocol::types::{Method, Request};

#[derive(Debug, Default, Clone)]
pub struct Manager;

impl Manager {
    pub fn parse(&self, line: &str) -> Option<Message> {
        if let Ok(request) = serde_json::from_str::<Request>(line) {
            let parsed = match request.method {
                Method::SetDifficulty => self.handle_set_difficulty(request),
                Method::Authorize => self.handle_authorize(request),
                // Method::Subscribe => self.handle_subscribe(request),
                Method::Submit => self.handle_submit(request),
                Method::Notify => self.handle_notify(request),
                _ => None,
            };

            if let Some(parsed) = parsed {
                return Some(parsed);
            }
        }

        None
    }

    fn handle_set_difficulty(&self, request: Request) -> Option<Message> {
        if let Some(params) = request.params {
            if let Some(params) = params.as_array() {
                if let Some(Some(difficulty)) = params.get(0).map(|param| param.as_f64()) {
                    return Some(Message::SetDifficulty { difficulty });
                }
            }
        }

        None
    }

    fn handle_authorize(&self, request: Request) -> Option<Message> {
        if let Some(params) = request.params {
            if let Some(params) = params.as_array() {
                let mut password = None;

                if let Some(Some(p)) = params.get(1).map(|param| param.as_str()) {
                    password = Some(p.trim().to_owned());
                }

                if let Some(Some(cred)) = params.get(0).map(|param| param.as_str()) {
                    let cred = cred.trim().to_lowercase();

                    if let Some((user, worker)) = cred.split_once('.') {
                        return Some(Message::Authenticate {
                            id: request.id.unwrap_or(999),
                            user: user.to_owned(),
                            worker: worker.replace(".", "").replace("-", "").replace("_", ""),
                            password,
                        });
                    } else {
                        let user = cred.replace("-", "").replace("_", "");

                        return Some(Message::Authenticate {
                            id: request.id.unwrap_or(999),
                            user,
                            worker: "default".to_owned(),
                            password,
                        });
                    }
                }
            }
        }

        None
    }

    fn handle_notify(&self, request: Request) -> Option<Message> {
        if let Some(params) = request.params {
            if let Some(params) = params.as_array() {
                if let Some(Some(job_id)) = params.get(0).map(|param| param.as_str()) {
                    return Some(Message::Notify {
                        job_id: job_id.to_owned(),
                    });
                }
            }
        }

        None
    }

    fn handle_submit(&self, request: Request) -> Option<Message> {
        if let Some(params) = request.params {
            if let Some(params) = params.as_array() {
                if let Some(Some(job_id)) = params.get(1).map(|param| param.as_str()) {
                    return Some(Message::Submit {
                        id: request.id.unwrap_or(998),
                        job_id: job_id.to_owned(),
                    });
                }
            }
        }

        None
    }
}
