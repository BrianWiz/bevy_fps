use shared::bevy::prelude::*;
use shared::bevy_rapier3d::plugin::RapierContext;
use shared::character::*;

use crate::net::Application;

pub fn move_system(
    fixed_time: Res<Time<Fixed>>,
    mut game_server: ResMut<Application>,
    mut physics: ResMut<RapierContext>,
    mut characters: Query<(&mut CharacterState, &mut Transform, &CharacterConstants)>,
) {
    for (mut char_state, mut char_xform, char_constants) in characters.iter_mut() {
        if let Some(client_info) = game_server
            .clients
            .iter_mut()
            .find(|c| c.client_id == char_state.owner_client_id)
        {
            if let Some(input_to_process) = &client_info.input_to_process {
                move_character(
                    input_to_process.compute_wish_dir(),
                    &mut physics,
                    &mut char_state,
                    &mut char_xform,
                    char_constants,
                    fixed_time.delta_seconds(),
                );

                client_info.server_last_processed_input_id = Some(input_to_process.id);
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
