use std::collections::VecDeque;

use bevy::{log::LogPlugin, prelude::*};
use bevy_quinnet::{server::QuinnetServerPlugin, shared::ClientId};
use shared::PlayerInput;

mod input;
mod net;

const TICKRATE_HZ: u64 = 64;

#[derive(Resource, Default)]
struct ServerState {
    pub tick: u64,
    pub clients: Vec<ClientInfo>,
}
struct ClientInfo {
    pub client_id: ClientId,
    pub inputs: VecDeque<PlayerInput>,
    pub last_processed_input: Option<PlayerInput>,
}

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(LogPlugin::default())
        .add_plugins(QuinnetServerPlugin::default())
        .add_systems(Startup, (net::s_start_listening).chain())
        .insert_resource(Time::<Fixed>::from_hz(TICKRATE_HZ as f64))
        .add_systems(
            FixedUpdate,
            (
                net::s_client_disconnected_system,
                net::s_handle_client_messages,
                input::s_consume_inputs,
                net::s_send_snapshot,
            )
                .chain(),
        )
        .insert_resource(ServerState::default())
        .run();
}
