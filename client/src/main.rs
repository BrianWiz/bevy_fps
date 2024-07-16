use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use bevy_quinnet::client::client_connected;
use bevy_quinnet::client::QuinnetClientPlugin;
use bevy_quinnet::shared::ClientId;
use shared::PlayerInput;
use shared::ServerSnapshot;
use std::collections::VecDeque;
use std::time::Duration;

mod input;
mod net;

const TICKRATE_HZ: u64 = 64;

pub struct SnapshotBuffer {
    pub snapshots: VecDeque<ServerSnapshot>,
}

impl Default for SnapshotBuffer {
    fn default() -> Self {
        SnapshotBuffer {
            snapshots: VecDeque::new(),
        }
    }
}

#[derive(Default, Resource)]
pub struct PlayerController {
    pub client_id: ClientId,
    pub latest_input: PlayerInput,
    pub input_history: Vec<PlayerInput>,
    pub next_input_id: u64,
    pub is_replaying: bool,
    pub last_processed_snapshot_tick: u64,
    pub snapshot_buffer: SnapshotBuffer,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(QuinnetClientPlugin::default())
        .add_systems(Startup, (setup, net::s_start_connection).chain())
        .add_systems(
            Update,
            (net::log_network_metrics.run_if(on_timer(Duration::from_secs(5))),).chain(),
        )
        .insert_resource(Time::<Fixed>::from_hz(TICKRATE_HZ as f64))
        .add_systems(
            FixedUpdate,
            (
                net::s_handle_server_messages,
                net::s_consume_snapshot_buffer,
                input::s_gather_movement_input,
                input::s_consume_input,
                input::s_send_input,
            )
                .chain()
                .run_if(client_connected),
        )
        .init_resource::<PlayerController>()
        .init_resource::<net::NetworkMetrics>()
        .run();
}

fn setup(mut commands: Commands) {}
