use shared::avian3d::spatial_query::SpatialQuery;
use shared::bevy::prelude::*;
use shared::character::*;
use shared::resources::DataAssetHandles;
use shared::weapons::{get_weapon_config, WeaponConfig, WeaponState};

use crate::net::Application;

pub fn consume_input_system(
    fixed_time: Res<Time<Fixed>>,
    data_asset_handles: Res<DataAssetHandles>,
    weapon_configs: Res<Assets<WeaponConfig>>,
    spatial_query: SpatialQuery,
    mut game_server: ResMut<Application>,
    mut characters: Query<(
        &mut CharacterState,
        &mut Transform,
        &CharacterConstants,
        &mut WeaponState,
    )>,
) {
    for (mut char_state, mut char_xform, char_constants, mut weapon_state) in characters.iter_mut()
    {
        if let Some(client_info) = game_server
            .clients
            .iter_mut()
            .find(|c| c.client_id == char_state.owner_client_id)
        {
            if client_info.inputs_to_process.is_empty() {
                move_character(
                    client_info.last_processed_input.compute_wish_dir(),
                    client_info.last_processed_input.jump,
                    &spatial_query,
                    &mut char_state,
                    &mut char_xform,
                    char_constants,
                    fixed_time.delta_seconds(),
                );
            } else {
                let chopped_delta =
                    fixed_time.delta_seconds() / client_info.inputs_to_process.len() as f32;
                for input in client_info.inputs_to_process.iter() {
                    move_character(
                        input.compute_wish_dir(),
                        input.jump,
                        &spatial_query,
                        &mut char_state,
                        &mut char_xform,
                        char_constants,
                        chopped_delta,
                    );

                    if input.fire && weapon_state.can_fire(&fixed_time) {
                        let weapon_config = get_weapon_config(
                            &data_asset_handles,
                            &weapon_configs,
                            &weapon_state.weapon_config_tag,
                        );

                        if let Some(weapon_config) = weapon_config {
                            weapon_state.on_fire(&fixed_time, weapon_config);
                            println!("Firing weapon!");
                        }
                    }
                    client_info.last_processed_input = input.clone();
                    client_info.server_last_processed_input_id = Some(input.id);
                }
                client_info.inputs_to_process.clear();
            }
        }
    }
}

pub fn despawn_system(
    mut commands: Commands,
    mut characters: Query<(Entity, &CharacterState)>,
    mut character_despawn_events: EventReader<CharacterDespawnEvent>,
) {
    for event in character_despawn_events.read() {
        for (entity, char_state) in characters.iter_mut() {
            if char_state.owner_client_id == event.client_id {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}
