use serde_json::{json, Value};

pub struct MockResponses;

impl MockResponses {
    pub fn subscribe_response(extranonce1: &str, extranonce2_size: usize) -> Value {
        json!({
            "id": 1,
            "result": [
                [
                    ["mining.set_difficulty", "subscription_id_1"],
                    ["mining.notify", "subscription_id_2"]
                ],
                extranonce1,
                extranonce2_size
            ],
            "error": null
        })
    }
    
    pub fn authorize_response(id: u64, authorized: bool) -> Value {
        json!({
            "id": id,
            "result": authorized,
            "error": if authorized { 
                Value::Null 
            } else { 
                json!(["Unauthorized worker", 25]) 
            }
        })
    }
    
    pub fn notify_message(job_params: Vec<Value>) -> Value {
        json!({
            "id": null,
            "method": "mining.notify",
            "params": job_params
        })
    }
    
    pub fn set_difficulty_message(difficulty: u64) -> Value {
        json!({
            "id": null,
            "method": "mining.set_difficulty",
            "params": [difficulty]
        })
    }
    
    pub fn submit_response(id: u64, accepted: bool, error: Option<String>) -> Value {
        json!({
            "id": id,
            "result": accepted,
            "error": error.map(|e| json!([e, 20]))
        })
    }
    
    pub fn extranonce_subscribe_response(id: u64) -> Value {
        json!({
            "id": id,
            "result": true,
            "error": null
        })
    }
    
    pub fn ping_response(id: u64) -> Value {
        json!({
            "id": id,
            "result": "pong",
            "error": null
        })
    }
    
    pub fn error_response(id: Option<u64>, error_msg: &str, error_code: i32) -> Value {
        json!({
            "id": id,
            "result": null,
            "error": [error_msg, error_code]
        })
    }
    
    pub fn get_version_response(id: u64) -> Value {
        json!({
            "id": id,
            "result": "MockPool/1.0.0",
            "error": null
        })
    }
    
    pub fn unknown_method_response(id: u64, method: &str) -> Value {
        json!({
            "id": id,
            "result": null,
            "error": [format!("Unknown method: {}", method), 20]
        })
    }
}