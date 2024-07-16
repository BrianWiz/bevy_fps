use crate::{input::s_consume_input, PlayerController};
use bevy::{ecs::system::RunSystemOnce, prelude::*};
use bevy_quinnet::{
    client::{
        certificate::CertificateVerificationMode, connection::ClientEndpointConfiguration,
        QuinnetClient,
    },
    shared::channels::ChannelsConfiguration,
};
use shared::{spawn_character, CharacterSnapshot, CharacterState, ServerMessage, ServerSnapshot};

pub fn is_locally_controlled(state: &CharacterState, our_client_id: u64) -> bool {
    state.owner_client_id == our_client_id
}

pub fn s_start_connection(mut client: ResMut<QuinnetClient>) {
    client
        .open_connection(
            ClientEndpointConfiguration::from_strings("127.0.0.1:6000", "0.0.0.0:0").unwrap(),
            CertificateVerificationMode::SkipVerification,
            ChannelsConfiguration::default(),
        )
        .unwrap();
}

pub fn s_handle_server_messages(
    mut client: ResMut<QuinnetClient>,
    mut controller: ResMut<PlayerController>,
) {
    while let Some((_, message)) = client
        .connection_mut()
        .try_receive_message::<ServerMessage>()
    {
        match message {
            ServerMessage::ServerSnapshot(snapshot) => {
                if snapshot.tick > controller.latest_input.server_tick {
                    controller.snapshot_buffer.snapshots.push_back(snapshot);
                }
            }
        }
    }
}

pub fn s_consume_snapshot_buffer(world: &mut World) {
    let mut client_id = 0;
    let mut time = Time::default();

    world.resource_scope(|_, client: Mut<QuinnetClient>| {
        if let Some(id) = client.connection().client_id() {
            client_id = id;
        }
    });

    world.resource_scope(|_, time_res: Mut<Time>| {
        time = time_res.clone();
    });

    let mut snapshot = None;
    world.resource_scope(|_, mut controller: Mut<PlayerController>| {
        snapshot = controller.snapshot_buffer.snapshots.pop_front();

        if let Some(snap) = &snapshot {
            if controller.latest_input.server_tick >= snap.tick {
                snapshot = None; // Skip this snapshot if it's too old
            } else {
                controller.latest_input.server_tick = snap.tick;
                controller.client_id = client_id;
            }
        }
    });

    if let Some(snapshot) = snapshot {
        process_snapshot(world, snapshot, client_id);
    }
}

fn process_snapshot(world: &mut World, snapshot: ServerSnapshot, client_id: u64) {
    for char_snap in &snapshot.character_snapshots {
        let mut locally_controlled = false;

        // Find and update or spawn character
        let character = world
            .query::<(&mut CharacterState, &mut Transform)>()
            .iter_mut(world)
            .find(|(char, _)| char.owner_client_id == char_snap.owner_client_id);

        if let Some((mut char_state, mut transform)) = character {
            update_character(&mut char_state, &mut transform, char_snap);
            locally_controlled = is_locally_controlled(&char_state, client_id);
        } else {
            spawn_character(&mut world.commands(), char_snap);
            world.flush();
        }

        if locally_controlled {
            replay_inputs(world, snapshot.tick, &char_snap, snapshot.acked_input_id);
        }
    }
}

fn update_character(
    char_state: &mut CharacterState,
    transform: &mut Transform,
    char_snap: &CharacterSnapshot,
) {
    char_state.velocity = char_snap.velocity.unwrap_or(char_state.velocity);
    transform.translation = char_snap.position.unwrap_or(transform.translation);
}

fn replay_inputs(
    world: &mut World,
    tick: u64,
    snapshot: &CharacterSnapshot,
    acked_input_id: Option<u64>,
) {
    let inputs_to_replay = {
        let mut inputs = Vec::new();
        world.resource_scope(|world, controller: Mut<PlayerController>| {
            if let Some(acked_id) = acked_input_id {
                if let Some(acked_input) = controller
                    .input_history
                    .iter()
                    .find(|input| input.id == acked_id)
                {
                    let diff = acked_input.final_position - snapshot.position.unwrap_or(Vec3::ZERO);
                    let diff_magnitude = diff.length();

                    world.resource_scope(|_, mut metrics: Mut<NetworkMetrics>| {
                        metrics.total_diff += diff_magnitude;
                        metrics.max_diff = metrics.max_diff.max(diff_magnitude);
                        metrics.diff_count += 1;
                    });

                    info!(
                        "Tick: {} - Diff: {} - Ours: {} - Theirs: {}",
                        tick,
                        diff,
                        acked_input.final_position,
                        snapshot.position.unwrap_or(Vec3::ZERO)
                    );
                }

                inputs = controller
                    .input_history
                    .iter()
                    .filter(|input| input.id > acked_id)
                    .cloned()
                    .collect();
            } else {
                println!("No acked input ID in snapshot");
            }
        });

        inputs
    };

    let latest_input = world
        .resource_scope(|_, controller: Mut<PlayerController>| controller.latest_input.clone());

    for input in inputs_to_replay {
        world.resource_scope(|_, mut controller: Mut<PlayerController>| {
            controller.is_replaying = true;
            controller.latest_input = input.clone();
        });
        world.run_system_once(s_consume_input);
    }

    world.resource_scope(|_, mut controller: Mut<PlayerController>| {
        controller.is_replaying = false;
        controller.latest_input = latest_input;
    });
}

#[derive(Default, Resource)]
pub struct NetworkMetrics {
    pub total_diff: f32,
    pub max_diff: f32,
    pub diff_count: u32,
}

// Periodically (e.g., every 5 seconds), log and reset these metrics
pub fn log_network_metrics(mut metrics: ResMut<NetworkMetrics>) {
    if metrics.diff_count > 0 {
        let avg_diff = metrics.total_diff / metrics.diff_count as f32;
        info!(
            "Network Metrics - Avg Diff: {}, Max Diff: {}",
            avg_diff, metrics.max_diff
        );
        *metrics = NetworkMetrics::default(); // Reset metrics
    }
}
