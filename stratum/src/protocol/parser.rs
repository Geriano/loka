use crate::error::Result;
use crate::protocol::messages::StratumMessage;
use crate::protocol::types::{Method, Request};

#[derive(Debug, Default, Clone)]
pub struct StratumParser;

impl StratumParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_message(&self, line: &str) -> Result<Option<StratumMessage>> {
        if let Ok(request) = serde_json::from_str::<Request>(line) {
            let parsed = match request.method {
                Method::Subscribe => self.handle_subscribe(request)?,
                Method::SetDifficulty => self.handle_set_difficulty(request)?,
                Method::Authorize => self.handle_authorize(request)?,
                Method::Submit => self.handle_submit(request)?,
                Method::Notify => self.handle_notify(request)?,
                _ => return Ok(None),
            };

            return Ok(Some(parsed));
        }

        Ok(None)
    }

    fn handle_subscribe(&self, request: Request) -> Result<StratumMessage> {
        let user_agent = if let Some(params) = request.params {
            if let Some(params) = params.as_array() {
                params.first()
                    .and_then(|param| param.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        } else {
            None
        };

        Ok(StratumMessage::Subscribe {
            id: request.id.unwrap_or(1),
            user_agent,
        })
    }

    fn handle_set_difficulty(&self, request: Request) -> Result<StratumMessage> {
        if let Some(params) = request.params {
            if let Some(params) = params.as_array() {
                if let Some(Some(difficulty)) = params.first().map(|param| param.as_f64()) {
                    return Ok(StratumMessage::SetDifficulty { difficulty });
                }
            }
        }

        Err(crate::error::StratumError::Protocol {
            message: "Invalid set_difficulty parameters".to_string(),
            method: Some("mining.set_difficulty".to_string()),
            request_id: None,
        })
    }

    fn handle_authorize(&self, request: Request) -> Result<StratumMessage> {
        if let Some(params) = request.params {
            if let Some(params) = params.as_array() {
                let mut password = None;

                if let Some(Some(p)) = params.get(1).map(|param| param.as_str()) {
                    password = Some(p.trim().to_owned());
                }

                if let Some(Some(cred)) = params.first().map(|param| param.as_str()) {
                    let (user, worker) = self.parse_credentials(cred)?;

                    return Ok(StratumMessage::Authenticate {
                        id: request.id.unwrap_or(999),
                        user,
                        worker,
                        password,
                    });
                }
            }
        }

        Err(crate::error::StratumError::Protocol {
            message: "Invalid authorize parameters".to_string(),
            method: Some("mining.authorize".to_string()),
            request_id: None,
        })
    }

    fn handle_notify(&self, request: Request) -> Result<StratumMessage> {
        if let Some(params) = request.params {
            if let Some(params) = params.as_array() {
                if let Some(Some(job_id)) = params.first().map(|param| param.as_str()) {
                    return Ok(StratumMessage::Notify {
                        job_id: job_id.to_owned(),
                    });
                }
            }
        }

        Err(crate::error::StratumError::Protocol {
            message: "Invalid notify parameters".to_string(),
            method: Some("mining.notify".to_string()),
            request_id: None,
        })
    }

    fn handle_submit(&self, request: Request) -> Result<StratumMessage> {
        if let Some(params) = request.params {
            if let Some(params) = params.as_array() {
                if let Some(Some(job_id)) = params.get(1).map(|param| param.as_str()) {
                    return Ok(StratumMessage::Submit {
                        id: request.id.unwrap_or(998),
                        job_id: job_id.to_owned(),
                    });
                }
            }
        }

        Err(crate::error::StratumError::Protocol {
            message: "Invalid submit parameters".to_string(),
            method: Some("mining.submit".to_string()),
            request_id: None,
        })
    }

    fn parse_credentials(&self, cred: &str) -> Result<(String, String)> {
        let cred = cred.trim().to_lowercase();

        if let Some((user, worker)) = cred.split_once('.') {
            Ok((user.to_owned(), self.clean_worker_name(worker)))
        } else {
            Ok((self.clean_worker_name(&cred), "default".to_owned()))
        }
    }

    fn clean_worker_name(&self, worker: &str) -> String {
        worker.replace(".", "").replace("-", "").replace("_", "")
    }
}
