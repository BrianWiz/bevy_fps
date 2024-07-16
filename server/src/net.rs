use avian3d::spatial_query::SpatialQuery;
use bevy::prelude::*;
use bevy_quinnet::{
    server::{
        certificate::CertificateRetrievalMode, ConnectionLostEvent, QuinnetServer,
        ServerEndpointConfiguration,
    },
    shared::{
        channels::{ChannelId, ChannelType, ChannelsConfiguration},
        ClientId,
    },
};
use shared::{
    character::{move_character, spawn_character, CharacterConstants, CharacterState},
    input::{compute_wish_dir, PlayerInput},
    snapshot::{CharacterSnapshot, ServerSnapshot},
    ClientMessage, ServerMessage,
};
use std::collections::VecDeque;

/// The minimum number of inputs that must be in the buffer before we start consuming them
const INPUT_BUFFER_MIN_SIZE: usize = 5;

/// The maximum number of inputs that can be in the buffer before we start dropping them
const INPUT_BUFFER_MAX_SIZE: usize = 10;

#[derive(Component)]
pub struct InputBuffer {
    pub buffer: VecDeque<PlayerInput>,
    pub last_processed_input_id: Option<u64>,
}

impl Default for InputBuffer {
    fn default() -> Self {
        Self {
            buffer: VecDeque::with_capacity(INPUT_BUFFER_MAX_SIZE),
            last_processed_input_id: None,
        }
    }
}

#[derive(Resource, Default)]
pub struct ServerState {
    pub tick: u64,
    pub clients: Vec<ClientInfo>,
    pub snapshot_history: Vec<ServerSnapshot>,
}
pub struct ClientInfo {
    pub client_id: ClientId,
    pub input_buffer: InputBuffer,
    pub last_processed_input: Option<PlayerInput>,
}

#[repr(u8)]
pub enum ServerChannels {
    SnapshotDiff,
    SnapshotFull,
    ImportantData,
}

impl Into<ChannelId> for ServerChannels {
    fn into(self) -> ChannelId {
        self as ChannelId
    }
}

impl ServerChannels {
    pub fn channels_configuration() -> ChannelsConfiguration {
        ChannelsConfiguration::from_types(vec![
            ChannelType::Unreliable,
            ChannelType::UnorderedReliable,
            ChannelType::UnorderedReliable,
        ])
        .unwrap()
    }
}

pub fn s_handle_client_messages(
    mut server: ResMut<QuinnetServer>,
    mut server_state: ResMut<ServerState>,
    mut commands: Commands,
) {
    let endpoint = server.endpoint_mut();
    for client_id in endpoint.clients() {
        while let Some((_, message)) = endpoint.try_receive_message_from::<ClientMessage>(client_id)
        {
            match message {
                ClientMessage::PlayerInput(input) => {
                    if let Some(client_info) = server_state
                        .clients
                        .iter_mut()
                        .find(|c| c.client_id == client_id)
                    {
                        add_input_to_buffer(&mut client_info.input_buffer, input);
                    } else {
                        let mut new_client_info = ClientInfo {
                            client_id,
                            input_buffer: InputBuffer::default(),
                            last_processed_input: None,
                        };
                        add_input_to_buffer(&mut new_client_info.input_buffer, input);
                        server_state.clients.push(new_client_info);

                        spawn_character(
                            &mut commands,
                            &CharacterSnapshot {
                                owner_client_id: client_id,
                                position: Some(Vec3::new(0.0, 2.0, 0.0)),
                                velocity: Some(Vec3::ZERO),
                            },
                        );
                    }
                }
            }
        }
    }
}

fn add_input_to_buffer(input_buffer: &mut InputBuffer, new_input: PlayerInput) {
    // find the correct position to insert the new input
    let insert_pos = input_buffer
        .buffer
        .iter()
        .position(|input| input.id > new_input.id)
        .unwrap_or(input_buffer.buffer.len()); // or put it at the end

    // only insert if it's a new input
    if insert_pos == 0 || new_input.id > input_buffer.buffer[insert_pos - 1].id {
        input_buffer.buffer.insert(insert_pos, new_input);

        if input_buffer.buffer.len() > INPUT_BUFFER_MAX_SIZE {
            input_buffer.buffer.pop_front();
        }
    }
}

pub fn s_start_listening(mut server: ResMut<QuinnetServer>) {
    server
        .start_endpoint(
            ServerEndpointConfiguration::from_string("0.0.0.0:6000").unwrap(),
            CertificateRetrievalMode::GenerateSelfSigned {
                server_hostname: "127.0.0.1".to_string(),
            },
            ServerChannels::channels_configuration(),
        )
        .unwrap();
}

pub fn s_client_disconnected_system(
    mut server_state: ResMut<ServerState>,
    mut events: EventReader<ConnectionLostEvent>,
) {
    for client in events.read() {
        server_state.clients.retain(|c| c.client_id != client.id);
        info!("Client disconnected: {}", client.id);
    }
}

pub fn s_send_snapshot(
    mut server: ResMut<QuinnetServer>,
    characters: Query<(&CharacterState, &Transform)>,
    mut server_state: ResMut<ServerState>,
) {
    let snapshot = ServerSnapshot {
        tick: server_state.tick,
        acked_input_id: None, // gets filled right before sending, per client
        character_snapshots: characters
            .iter()
            .map(|(char_state, transform)| CharacterSnapshot {
                owner_client_id: char_state.owner_client_id,
                position: Some(transform.translation),
                velocity: Some(char_state.velocity),
            })
            .collect(),
    };

    server_state.tick += 1;
    server_state.snapshot_history.push(snapshot.clone());

    let endpoint = server.endpoint_mut();
    for client_id in &server_state.clients {
        if let Some(last_processed_input) = &client_id.last_processed_input {
            if let Some(mut snap) = if let Some(old) = server_state
                .snapshot_history
                .iter()
                .find(|snap| snap.tick == last_processed_input.server_tick)
            {
                Some(snapshot.diff(old))
            } else {
                None
            } {
                snap.acked_input_id = Some(last_processed_input.id);
                if let Err(err) = endpoint.send_message_on(
                    client_id.client_id,
                    ServerChannels::SnapshotDiff,
                    ServerMessage::ServerSnapshot(snap),
                ) {
                    error!("Error sending diffed snapshot with acked input: {:?}", err);
                }
            } else {
                let mut snapshot = snapshot.clone();
                snapshot.acked_input_id = Some(last_processed_input.id);
                if let Err(err) = endpoint.send_message_on(
                    client_id.client_id,
                    ServerChannels::SnapshotFull,
                    ServerMessage::ServerSnapshot(snapshot),
                ) {
                    error!("Error sending full snapshot with acked input: {:?}", err);
                }
            }
        } else {
            let mut snapshot = snapshot.clone();
            snapshot.acked_input_id = None;
            if let Err(err) = endpoint.send_message_on(
                client_id.client_id,
                ServerChannels::SnapshotFull,
                ServerMessage::ServerSnapshot(snapshot),
            ) {
                error!("Error sending full snapshot without acked input: {:?}", err);
            }
        }
    }
}

pub fn s_consume_inputs(
    spatial_query: SpatialQuery,
    mut server_state: ResMut<ServerState>,
    mut characters: Query<(&mut CharacterState, &CharacterConstants, &mut Transform)>,
    fixed_time: Res<Time<Fixed>>,
) {
    for client_info in server_state.clients.iter_mut() {
        let char_query = characters
            .iter_mut()
            .find(|(char_state, _, _)| char_state.owner_client_id == client_info.client_id);

        // buffer must build up a bit first
        if client_info.input_buffer.buffer.len() < INPUT_BUFFER_MIN_SIZE {
            continue;
        }

        if let Some((mut char_state, char_consts, mut transform)) = char_query {
            if let Some(input) = client_info.input_buffer.buffer.pop_front() {
                move_character(
                    compute_wish_dir(&input),
                    input.jump,
                    &spatial_query,
                    &mut char_state,
                    &mut transform,
                    &char_consts,
                    fixed_time.delta_seconds(),
                );

                client_info.input_buffer.last_processed_input_id = Some(input.id);
                client_info.last_processed_input = Some(input);
            }
        }
    }
}
