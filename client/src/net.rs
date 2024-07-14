use std::thread::sleep;
use std::time::Duration;

use crate::character::{self, spawn_character};
use crate::input::PlayerInputController;
use shared::bevy::ecs::system::RunSystemOnce;
use shared::bevy::prelude::*;
use shared::bevy_quinnet::client::certificate::CertificateVerificationMode;
use shared::bevy_quinnet::client::connection::{
    ClientEndpointConfiguration, ConnectionEvent, ConnectionFailedEvent,
};
use shared::bevy_quinnet::client::QuinnetClient;
use shared::character::{CharacterDespawnEvent, CharacterState};
use shared::protocol::{ClientChannels, ClientMessage, ServerMessage};
use shared::resources::DataAssetHandles;

pub fn handle_received_messages_system(world: &mut World) {
    world.resource_scope(|world, mut client: Mut<QuinnetClient>| {
        let client_id = client.connection().client_id().unwrap_or(0);
        let endpoint = client.connection_mut();

        while let Some(message) = endpoint.try_receive_message::<ServerMessage>() {
            match message {
                // we received a list of weapon configs, add them as assets
                (_channel_id, ServerMessage::WeaponConfig(weapon_config)) => {
                    world.resource_scope(|world, asset_server: Mut<AssetServer>| {
                        world.resource_scope(|_, mut data_asset_handles: Mut<DataAssetHandles>| {
                            // build up new assets
                            shared::bevy::log::info!("Received weapon config: {:?}", weapon_config);
                            data_asset_handles
                                .weapon_configs
                                .insert(weapon_config.tag.clone(), asset_server.add(weapon_config));
                        });
                    });
                }

                // we received a snapshot of the game state
                (_channel_id, ServerMessage::TickSnapshot(snapshot)) => {
                    // query for existing characters
                    for char_snap in &snapshot.characters {
                        // lc_ = locally controlled
                        let mut should_spawn_new = true;
                        let mut locally_controlled = false;
                        let mut lc_position_before = Vec3::ZERO;
                        let mut lc_entity = None;

                        if let Some((entity, mut existing_char_state, mut existing_char_xform)) =
                            world
                                .query::<(Entity, &mut CharacterState, &mut Transform)>()
                                .iter_mut(world)
                                .find(|(_, char, _)| {
                                    char.owner_client_id == char_snap.owner_client_id
                                })
                        {
                            should_spawn_new = false;

                            if existing_char_state.is_locally_controlled(client_id) {
                                locally_controlled = true;
                                lc_position_before = existing_char_xform.translation.clone();
                                lc_entity = Some(entity);
                            }

                            existing_char_state
                                .apply_snapshot(&char_snap, &mut existing_char_xform);
                        }

                        if locally_controlled {
                            if char_snap.position.is_none() {
                                continue;
                            }

                            // we are the owner of this character
                            // so we need to replay inputs since the last acked input the server has provided us
                            if let Some(acked_input_id) = snapshot.acked_input_id {
                                let acked_input = if let Some(input_controller) =
                                    world.get_resource::<PlayerInputController>()
                                {
                                    input_controller.get_input(acked_input_id)
                                } else {
                                    None
                                };

                                if let Some(acked_input) = acked_input {
                                    println!(
                                        "Diff: {:?}",
                                        acked_input
                                            .final_position
                                            .distance(char_snap.position.unwrap())
                                    );
                                }

                                let inputs_to_replay = if let Some(input_controller) =
                                    world.get_resource::<PlayerInputController>()
                                {
                                    input_controller.inputs_after(acked_input_id)
                                } else {
                                    Vec::new()
                                };

                                for input in inputs_to_replay {
                                    if let Some(mut input_controller) =
                                        world.get_resource_mut::<PlayerInputController>()
                                    {
                                        input_controller.latest_input = input;
                                        world.run_system_once(character::move_system);
                                    }
                                }
                            }

                            if let Some(entity) = lc_entity {
                                if let Ok((mut char_state, xform)) = world
                                    .query::<(&mut CharacterState, &Transform)>()
                                    .get_mut(world, entity)
                                {
                                    let lc_position_after = xform.translation.clone();
                                    let diff = lc_position_after - lc_position_before;
                                    char_state.visuals_offset = diff;
                                }
                            }
                        }

                        if should_spawn_new {
                            world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
                                world.resource_scope(
                                    |world, mut materials: Mut<Assets<StandardMaterial>>| {
                                        let mut commands = world.commands();
                                        spawn_character(
                                            &mut meshes,
                                            &mut materials,
                                            &mut commands,
                                            char_snap.owner_client_id,
                                            &char_snap.position.unwrap_or(Vec3::ZERO),
                                            char_snap.owner_client_id == client_id,
                                        );
                                        world.flush();
                                    },
                                );
                            });
                        }
                    }

                    // handle deletions, any character that isn't in the snapshot should be deleted
                    let mut deletions = Vec::new();
                    for char_state in world.query::<&mut CharacterState>().iter(world) {
                        if snapshot.characters.iter().all(|char_snap| {
                            char_snap.owner_client_id != char_state.owner_client_id
                        }) {
                            deletions.push(char_state.owner_client_id);
                        }
                    }
                    for client_id in deletions {
                        world.send_event(CharacterDespawnEvent {
                            client_id: client_id,
                        });
                    }

                    // Ack the server tick/snapshot!
                    world.resource_scope(|_, mut input_controller: Mut<PlayerInputController>| {
                        input_controller.latest_input.server_tick = Some(snapshot.tick);
                    });
                }
            }
        }
    });
}

pub fn handle_client_events_system(
    mut connection_events: EventReader<ConnectionEvent>,
    mut connection_failed_events: EventReader<ConnectionFailedEvent>,
    client: ResMut<QuinnetClient>,
) {
    if !connection_events.is_empty() {
        // We are connected
        let username: String = "Unnamed Player".into();
        shared::bevy::log::info!("Connected to server. With username: {}", username);
        if let Err(err) = client
            .connection()
            .send_message_on(ClientChannels::Events, ClientMessage::Connect { username })
        {
            shared::bevy::log::error!("Failed to send join message: {:?}", err);
        }
        connection_events.clear();
    }
    for ev in connection_failed_events.read() {
        shared::bevy::log::error!("Connection failed: {:?}", ev.err);
    }
}

pub fn start_connection_system(mut client: ResMut<QuinnetClient>) {
    if let Err(err) = client.open_connection(
        ClientEndpointConfiguration::from_strings("127.0.0.1:7777", "0.0.0.0:0").unwrap(),
        CertificateVerificationMode::SkipVerification,
        ClientChannels::channels_configuration(),
    ) {
        shared::bevy::log::error!("Failed to open connection: {:?}", err);
    }
}

pub fn send_input_system(
    client: ResMut<QuinnetClient>,
    input_controller: Res<PlayerInputController>,
) {
    if let Err(err) = client.connection().send_message_on(
        ClientChannels::PlayerInputs,
        ClientMessage::PlayerInput(input_controller.latest_input.clone()),
    ) {
        shared::bevy::log::error!("Failed to send input: {:?}", err);
    }
}

pub fn on_app_exit_system(app_exit_events: EventReader<AppExit>, client: Res<QuinnetClient>) {
    if !app_exit_events.is_empty() {
        client
            .connection()
            .send_message(ClientMessage::Disconnect {})
            .unwrap();
        // TODO Clean: event to let the async client send his last messages.
        sleep(Duration::from_secs_f32(0.5));
    }
}
