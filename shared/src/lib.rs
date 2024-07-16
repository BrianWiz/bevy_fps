use input::PlayerInput;
use serde::{Deserialize, Serialize};
use snapshot::ServerSnapshot;

pub mod character;
pub mod input;
pub mod snapshot;

#[derive(Serialize, Deserialize, Clone)]
pub enum ClientMessage {
    PlayerInput(PlayerInput),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ServerMessage {
    ServerSnapshot(ServerSnapshot),
}
