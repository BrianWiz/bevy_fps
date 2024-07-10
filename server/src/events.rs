use shared::{bevy::prelude::*, protocol::PlayerInput};

#[derive(Event)]
pub struct ClientConnectedEvent {
    pub client_id: u64,
    pub username: String,
}

#[derive(Event)]
pub struct ClientDisconnectedEvent {
    pub client_id: u64,
}

#[derive(Event)]
pub struct ClientInputEvent {
    pub client_id: u64,
    pub input: PlayerInput,
}
