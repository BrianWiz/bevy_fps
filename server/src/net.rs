use bevy::prelude::*;
use bevy_quinnet::{
    server::{
        certificate::CertificateRetrievalMode, ConnectionLostEvent, QuinnetServer,
        ServerEndpointConfiguration,
    },
    shared::{channels::ChannelsConfiguration, ClientId},
};
use shared::{
    compute_wish_dir, spawn_character, CharacterConstants, CharacterSnapshot, CharacterState,
    ClientMessage, PlayerInput, ServerMessage,
};
use std::collections::VecDeque;

const MAX_BUFFER_SIZE: usize = 10; // Adjust this value as needed

#[derive(Component)]
pub struct InputBuffer {
    pub buffer: VecDeque<PlayerInput>,
    pub last_processed_input_id: Option<u64>,
}

impl Default for InputBuffer {
    fn default() -> Self {
        Self {
            buffer: VecDeque::with_capacity(MAX_BUFFER_SIZE),
            last_processed_input_id: None,
        }
    }
}

#[derive(Resource, Default)]
pub struct ServerState {
    pub tick: u64,
    pub clients: Vec<ClientInfo>,
}
pub struct ClientInfo {
    pub client_id: ClientId,
    pub input_buffer: InputBuffer,
    pub last_processed_input: Option<PlayerInput>,
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
                                position: Some(Vec3::ZERO),
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
        .unwrap_or(input_buffer.buffer.len());

    // only insert if it's a new input
    if insert_pos == 0 || input_buffer.buffer[insert_pos - 1].id < new_input.id {
        input_buffer.buffer.insert(insert_pos, new_input);

        if input_buffer.buffer.len() > MAX_BUFFER_SIZE {
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
            ChannelsConfiguration::default(),
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
    let snapshot = shared::ServerSnapshot {
        tick: server_state.tick,
        acked_input_id: None, // gets filled in later
        character_snapshots: characters
            .iter()
            .map(|(char_state, transform)| shared::CharacterSnapshot {
                owner_client_id: char_state.owner_client_id,
                position: Some(transform.translation),
                velocity: Some(char_state.velocity),
            })
            .collect(),
    };

    let endpoint = server.endpoint_mut();
    for client_id in &server_state.clients {
        let mut snapshot = snapshot.clone();
        if let Some(last_processed_input) = &client_id.last_processed_input {
            snapshot.acked_input_id = Some(last_processed_input.id);
        }
        if let Err(err) = endpoint.send_message(
            client_id.client_id,
            ServerMessage::ServerSnapshot(snapshot.clone()),
        ) {
            error!("Error sending message: {:?}", err);
        }
    }

    server_state.tick += 1;
}

pub fn s_consume_inputs(
    mut server_state: ResMut<ServerState>,
    mut characters: Query<(&mut CharacterState, &CharacterConstants, &mut Transform)>,
    fixed_time: Res<Time<Fixed>>,
) {
    for client_info in server_state.clients.iter_mut() {
        let char_query = characters
            .iter_mut()
            .find(|(char_state, _, _)| char_state.owner_client_id == client_info.client_id);

        if let Some((_, char_consts, mut transform)) = char_query {
            if let Some(input) = client_info.input_buffer.buffer.pop_front() {
                let speed = char_consts.move_speed * fixed_time.delta_seconds();
                let wish_dir = compute_wish_dir(&input);
                transform.translation += wish_dir * speed;

                client_info.input_buffer.last_processed_input_id = Some(input.id);
                client_info.last_processed_input = Some(input);
            }
            // no input in the buffer? consume the last processed input
            else {
                if let Some(last_input) = &client_info.last_processed_input {
                    let speed = char_consts.move_speed * fixed_time.delta_seconds();
                    let wish_dir = compute_wish_dir(last_input);
                    transform.translation += wish_dir * speed;
                }
            }
        }
    }
}
