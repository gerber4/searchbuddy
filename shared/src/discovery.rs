use serde::{Deserialize, Serialize};
use std::net::SocketAddrV4;

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct Instance {
    pub instance_id: i32,
    pub address: SocketAddrV4,
}

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct RegisterRequest {
    pub listen_address: SocketAddrV4,
}

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct RegisterResponse {
    pub instance_id: i32,
}

#[derive(Copy, Clone, Deserialize, Serialize)]
pub enum PingResult {
    Ok,
    NoLongerActive,
}

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct PingRequest {
    pub listen_address: SocketAddrV4,
    pub instance_id: i32,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct PingResponse {
    pub ping_result: PingResult,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ChatroomRequest {
    pub term: String,
}

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct ChatroomResponse {
    pub instance: Option<Instance>,
}
