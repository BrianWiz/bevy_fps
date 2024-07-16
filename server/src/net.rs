use std::collections::VecDeque;

use crate::{ClientInfo, ServerState};
use bevy::prelude::*;
use bevy_quinnet::{
    server::{
        certificate::CertificateRetrievalMode, ConnectionLostEvent, QuinnetServer,
        ServerEndpointConfiguration,
    },
    shared::channels::ChannelsConfiguration,
};
use shared::{spawn_character, CharacterSnapshot, CharacterState, ClientMessage, ServerMessage};

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
                        client_info.inputs.push_back(input);
                    } else {
                        server_state.clients.push(ClientInfo {
                            client_id,
                            inputs: VecDeque::from(vec![input]),
                            last_processed_input: None,
                        });
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
