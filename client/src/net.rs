use std::collections::VecDeque;

use crate::{
    character::{spawn_visual_character, CharacterVisuals},
    TICKRATE_HZ,
};
use bevy::{ecs::system::RunSystemOnce, prelude::*};
use bevy_quinnet::{
    client::{
        certificate::CertificateVerificationMode, connection::ClientEndpointConfiguration,
        QuinnetClient,
    },
    shared::{
        channels::{ChannelId, ChannelType, ChannelsConfiguration},
        ClientId,
    },
};
use shared::{
    character::CharacterState,
    input::PlayerInput,
    snapshot::{CharacterSnapshot, ServerSnapshot},
    ClientMessage, ServerMessage,
};

/// The difference in magnitude between past position and current server position
/// that we consider a large correction for logging purposes.
const LOG_LARGE_CORRECTION_THRESHOLD: f32 = 0.9;

#[derive(Default, Resource)]
pub struct PlayerController {
    pub client_id: ClientId,
    pub latest_input: PlayerInput,
    pub input_history: Vec<PlayerInput>,
    pub next_input_id: u64,
    pub is_replaying: bool,
    pub latest_snapshot: Option<ServerSnapshot>,
}

#[derive(Default, Resource)]
pub struct NetworkMetrics {
    pub total_diff: f32,
    pub max_diff: f32,
    pub diff_count: u32,
}

#[repr(u8)]
pub enum ClientChannels {
    Events,
    PlayerInputs,
}
impl Into<ChannelId> for ClientChannels {
    fn into(self) -> ChannelId {
        self as ChannelId
    }
}
impl ClientChannels {
    pub fn channels_configuration() -> ChannelsConfiguration {
        ChannelsConfiguration::from_types(vec![
            ChannelType::OrderedReliable,
            ChannelType::Unreliable,
        ])
        .unwrap()
    }
}

pub fn is_locally_controlled(state: &CharacterState, our_client_id: u64) -> bool {
    state.owner_client_id == our_client_id
}

pub fn s_start_connection(mut client: ResMut<QuinnetClient>) {
    client
        .open_connection(
            ClientEndpointConfiguration::from_strings("127.0.0.1:6000", "0.0.0.0:0").unwrap(),
            CertificateVerificationMode::SkipVerification,
            ClientChannels::channels_configuration(),
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
            ServerMessage::ServerSnapshot(new_snapshot) => {
                if controller.latest_input.server_tick >= new_snapshot.tick {
                    continue;
                }
                controller.latest_snapshot = Some(new_snapshot);
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

    let mut snapshot = None::<ServerSnapshot>;
    world.resource_scope(|_, mut controller: Mut<PlayerController>| {
        // acks the snapshot
        if let Some(snap) = &controller.latest_snapshot {
            controller.latest_input.server_tick = snap.tick;
        }
        controller.client_id = client_id;
        snapshot = controller.latest_snapshot.take();
    });

    if let Some(snap) = snapshot {
        process_snapshot(world, &snap, client_id);
    }
}

fn process_snapshot(world: &mut World, snapshot: &ServerSnapshot, client_id: u64) {
    for char_snap in &snapshot.character_snapshots {
        // find and update or spawn character
        let character = world
            .query::<(&mut CharacterState, &mut Transform)>()
            .iter_mut(world)
            .find(|(char, _)| char.owner_client_id == char_snap.owner_client_id);

        if let Some((mut char_state, mut transform)) = character {
            update_character(&mut char_state, &mut transform, char_snap);
            if is_locally_controlled(&char_state, client_id) && char_snap.position.is_some() {
                replay_inputs(world, snapshot.tick, &char_snap, snapshot.acked_input_id);
            }
        } else {
            // need to spawn these instantly without delay
            world.resource_scope(|world, mut materials: Mut<Assets<StandardMaterial>>| {
                world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
                    spawn_visual_character(
                        &mut world.commands(),
                        &char_snap,
                        &mut materials,
                        &mut meshes,
                    );
                });
            });
            world.flush();
        }
    }
}

fn update_character(
    char_state: &mut CharacterState,
    transform: &mut Transform,
    char_snap: &CharacterSnapshot,
) {
    if let Some(vel) = char_snap.velocity {
        char_state.velocity = vel;
    }

    if let Some(pos) = char_snap.position {
        transform.translation = pos;
    }
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
                let mut diff = Vec3::ZERO;

                if let Some(acked_input) = controller
                    .input_history
                    .iter()
                    .find(|input| input.id == acked_id)
                {
                    diff = snapshot.position.unwrap_or(Vec3::ZERO) - acked_input.final_position;
                    let diff_magnitude = diff.length();

                    world.resource_scope(|_, mut metrics: Mut<NetworkMetrics>| {
                        metrics.total_diff += diff_magnitude;
                        metrics.max_diff = metrics.max_diff.max(diff_magnitude);
                        metrics.diff_count += 1;
                    });

                    if diff_magnitude > LOG_LARGE_CORRECTION_THRESHOLD {
                        warn!(
                            "Large correction! - Tick: {} - Diff: {} - Ours: {} - Theirs: {}",
                            tick,
                            diff,
                            acked_input.final_position,
                            snapshot.position.unwrap_or(Vec3::ZERO)
                        );
                    }
                }

                inputs = controller
                    .input_history
                    .iter()
                    .filter(|input| input.id > acked_id)
                    .cloned()
                    .collect();

                if let Some(mut vis) = world
                    .query::<&mut CharacterVisuals>()
                    .iter_mut(world)
                    .find(|vis| vis.owner_client_id == snapshot.owner_client_id)
                {
                    vis.correction_offset += diff;
                }
            } else {
                warn!("No acked input ID in snapshot");
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
        world.run_system_once(crate::input::s_consume_move_input);
    }

    world.resource_scope(|_, mut controller: Mut<PlayerController>| {
        controller.is_replaying = false;
        controller.latest_input = latest_input;
    });
}

pub fn s_send_input(mut client: ResMut<QuinnetClient>, mut controller: ResMut<PlayerController>) {
    // cache the latest input
    let input = controller.latest_input.clone();
    controller.input_history.push(input);

    // retain the last 3 seconds of inputs
    let oldest_input_id = controller.next_input_id.saturating_sub(TICKRATE_HZ * 3);
    controller
        .input_history
        .retain(|input| input.id >= oldest_input_id);

    // send the last two inputs to the server
    let inputs_to_send = controller
        .input_history
        .iter()
        .rev()
        .take(2)
        .cloned()
        .collect::<Vec<_>>();

    for input in inputs_to_send {
        if let Err(err) = client.connection_mut().send_message_on(
            ClientChannels::PlayerInputs,
            ClientMessage::PlayerInput(input.clone()),
        ) {
            error!("Error sending message: {:?}", err);
        }
    }
}

pub fn s_log_network_metrics(mut metrics: ResMut<NetworkMetrics>) {
    if metrics.diff_count > 0 {
        let avg_diff = metrics.total_diff / metrics.diff_count as f32;
        info!(
            "Network Metrics - Avg Move Diff: {}, Max Move Diff: {}",
            avg_diff, metrics.max_diff
        );
        *metrics = NetworkMetrics::default(); // Reset metrics
    }
}
