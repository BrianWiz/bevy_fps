use shared::bevy::asset::LoadedFolder;
use shared::bevy::prelude::*;
use shared::bevy_quinnet::server::certificate::CertificateRetrievalMode;
use shared::bevy_quinnet::server::ConnectionLostEvent;
use shared::bevy_quinnet::server::QuinnetServer;
use shared::bevy_quinnet::server::ServerEndpointConfiguration;
use shared::bevy_quinnet::shared::ClientId;
use shared::character::*;
use shared::protocol::*;
use shared::resources::DataAssetHandles;
use shared::weapons::WeaponConfig;
use std::any::TypeId;

use crate::events::ClientConnectedEvent;
use crate::events::ClientDisconnectedEvent;
use crate::events::ClientInputEvent;
use crate::DataFolder;
use crate::TICKRATE;

#[derive(Resource)]
pub struct Application {
    next_tick: u32,
    snapshot_history: Vec<TickSnapshot>,
    pub clients: Vec<ClientInfo>,
}
impl Default for Application {
    fn default() -> Self {
        Application {
            next_tick: 0,
            snapshot_history: Vec::new(),
            clients: Vec::new(),
        }
    }
}

pub struct ClientInfo {
    pub client_id: ClientId,
    pub input_to_process: Option<PlayerInput>,
    pub client_last_acked_tick: Option<u32>,
    pub server_last_processed_input_id: Option<u32>,
}

pub fn handle_client_connected_system(
    weapon_configs_assets: Res<Assets<WeaponConfig>>,
    mut server: ResMut<QuinnetServer>,
    mut app: ResMut<Application>,
    mut events: EventReader<ClientConnectedEvent>,
) {
    for event in events.read() {
        app.clients.push(ClientInfo {
            client_id: event.client_id,
            input_to_process: None,
            client_last_acked_tick: None,
            server_last_processed_input_id: None,
        });

        let weapon_config_list = WeaponConfigList {
            configs: weapon_configs_assets.iter().map(|a| a.1).cloned().collect(),
        };

        println!("{:?}", weapon_config_list);
        if let Err(err) = server.endpoint_mut().send_message_on(
            event.client_id,
            ServerChannels::ImportantData,
            ServerMessage::WeaponConfigList(weapon_config_list),
        ) {
            shared::bevy::log::error!(
                "Failed to send weapon config data to client ({}): {}",
                event.client_id,
                err
            );
        }

        shared::bevy::log::info!("Client connected ({}): {}", event.client_id, event.username);
    }
}

pub fn handle_client_input_system(
    mut app: ResMut<Application>,
    mut events: EventReader<ClientInputEvent>,
) {
    for event in events.read() {
        if let Some(client_info) = app
            .clients
            .iter_mut()
            .find(|c| c.client_id == event.client_id)
        {
            client_info.input_to_process = Some(event.input.clone());
            client_info.client_last_acked_tick = event.input.server_tick;
        }
    }
}

pub fn handle_client_disconnected_system(
    mut server: ResMut<QuinnetServer>,
    mut app: ResMut<Application>,
    mut events: EventReader<ClientDisconnectedEvent>,
) {
    for event in events.read() {
        app.clients.retain(|c| c.client_id != event.client_id);
        if let Err(err) = server.endpoint_mut().disconnect_client(event.client_id) {
            shared::bevy::log::error!("Failed to disconnect client: {:?}", err);
        } else {
            shared::bevy::log::info!("Client disconnected: {}", event.client_id);
        }
    }
}

pub fn handle_received_messages_system(
    mut server: ResMut<QuinnetServer>,
    mut input_events: EventWriter<ClientInputEvent>,
    mut connected_events: EventWriter<ClientConnectedEvent>,
    mut disconnected_events: EventWriter<ClientDisconnectedEvent>,
) {
    let endpoint = server.endpoint_mut();
    for client_id in endpoint.clients() {
        while let Some((_, message)) = endpoint.try_receive_message_from::<ClientMessage>(client_id)
        {
            match message {
                ClientMessage::PlayerInput(input) => {
                    input_events.send(ClientInputEvent { client_id, input });
                }
                ClientMessage::Connect { username } => {
                    connected_events.send(ClientConnectedEvent {
                        client_id,
                        username,
                    });
                }
                ClientMessage::Disconnect {} => {
                    disconnected_events.send(ClientDisconnectedEvent { client_id });
                }

                _ => {}
            }
        }
    }
}

pub fn handle_server_events_system(
    mut event_writer: EventWriter<CharacterDespawnEvent>,
    mut connection_lost_events: EventReader<ConnectionLostEvent>,
    mut app: ResMut<Application>,
) {
    for client in connection_lost_events.read() {
        app.clients.retain(|c| c.client_id != client.id);
        event_writer.send(CharacterDespawnEvent {
            client_id: client.id,
        });
        shared::bevy::log::info!("Client disconnected: {}", client.id);
    }
}

pub fn start_listening_system(mut server: ResMut<QuinnetServer>) {
    if let Err(err) = server.start_endpoint(
        ServerEndpointConfiguration::from_string("0.0.0.0:7777").unwrap(),
        CertificateRetrievalMode::GenerateSelfSigned {
            server_hostname: "127.0.0.1".to_string(),
        },
        ServerChannels::channels_configuration(),
    ) {
        shared::bevy::log::error!("Failed to start server: {:?}", err);
    }
}

pub fn snapshot_system(
    mut server: ResMut<QuinnetServer>,
    mut app: ResMut<Application>,
    mut characters: Query<(&CharacterState, &Transform)>,
) {
    let mut snapshot = TickSnapshot {
        tick: app.next_tick,
        acked_input_id: None, // gets filled in before sending to client
        characters: Vec::new(),
    };

    // capture the state of all characters
    for (char_state, char_xform) in characters.iter_mut() {
        snapshot.characters.push(CharacterSnapshot {
            owner_client_id: char_state.owner_client_id,
            position: Some(char_xform.translation),
            velocity: Some(char_state.velocity),
        });
    }

    // retain a history of 2 seconds worth of snapshots
    // so that we have something to diff against when sending snapshots to clients
    let oldest_tick = app.next_tick.saturating_sub(TICKRATE * 2);
    app.snapshot_history
        .retain(|snapshot| snapshot.tick > oldest_tick);

    // loop through all clients, get their last acked tick, diff against the current tick
    // and send the diff to the client
    let endpoint = server.endpoint_mut();
    for client_info in &app.clients {
        // this tells the client we acked their input
        snapshot.acked_input_id = client_info.server_last_processed_input_id;

        if let Some(last_acked_tick) = client_info.client_last_acked_tick {
            if let Some(last_acked_snapshot) = app
                .snapshot_history
                .iter()
                .find(|snapshot| snapshot.tick == last_acked_tick)
            {
                if let Err(err) = endpoint.send_message_on(
                    client_info.client_id,
                    ServerChannels::SnapshotDiff,
                    ServerMessage::TickSnapshot(snapshot.diff(last_acked_snapshot)),
                ) {
                    shared::bevy::log::error!("Failed to send snapshot to client: {:?}", err);
                    return;
                }
            }
        }
        // if we don't have a snapshot to diff against, send the full snapshot
        if let Err(err) = endpoint.send_message_on(
            client_info.client_id,
            ServerChannels::SnapshotDiff,
            ServerMessage::TickSnapshot(snapshot.clone()),
        ) {
            shared::bevy::log::error!("Failed to send snapshot to client: {:?}", err);
        }
    }

    app.snapshot_history.push(snapshot);
    app.next_tick += 1;
}

pub fn data_load_system(
    folders: Res<Assets<LoadedFolder>>,
    data_folder: Res<DataFolder>,
    mut data_asset_handles: ResMut<DataAssetHandles>,
    weapon_config_assets: Res<Assets<WeaponConfig>>,
    mut events: EventReader<AssetEvent<LoadedFolder>>,
    mut server: ResMut<QuinnetServer>,
    asset_server: Res<AssetServer>,
) {
    // listens to load state of the data folder and gets the weapon configs
    for event in events.read() {
        if let AssetEvent::LoadedWithDependencies { id: asset_id } = event {
            // build up the weapon config list
            let mut new_weapon_config_list = WeaponConfigList {
                configs: Vec::new(),
            };
            let type_id = TypeId::of::<WeaponConfig>();
            if &data_folder.0.id() == asset_id {
                if let Some(loaded_folder) = folders.get(&data_folder.0) {
                    for handle in &loaded_folder.handles {
                        if handle.type_id() == type_id {
                            let typed_handle: Handle<WeaponConfig> = handle.clone().typed();
                            if let Some(weapon_config) = weapon_config_assets.get(&typed_handle) {
                                new_weapon_config_list.configs.push(weapon_config.clone());
                            }
                        }
                    }
                }
            }

            if new_weapon_config_list.configs.len() > 0 {
                // replace handles with new ones
                data_asset_handles.weapon_configs.clear();
                data_asset_handles.weapon_configs = weapon_config_assets
                    .iter()
                    .map(|a| asset_server.get_id_handle(a.0).unwrap_or_default())
                    .collect();

                // sends the configs to all clients
                let clients = server.endpoint().clients();
                if let Err(err) = server.endpoint_mut().send_group_message_on(
                    clients.iter(),
                    ServerChannels::ImportantData,
                    ServerMessage::WeaponConfigList(new_weapon_config_list),
                ) {
                    shared::bevy::log::error!(
                        "Failed to send weapon config list to clients: {:?}",
                        err
                    );
                }
            }
        }
    }
}
