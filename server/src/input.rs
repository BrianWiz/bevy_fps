use crate::ServerState;
use bevy::prelude::*;
use shared::{compute_wish_dir, CharacterConstants, CharacterState};

pub fn s_consume_inputs(
    mut server_state: ResMut<ServerState>,
    mut characters: Query<(&mut CharacterState, &CharacterConstants, &mut Transform)>,
    fixed_time: Res<Time<Fixed>>,
) {
    for client_info in server_state.clients.iter_mut() {
        if client_info.inputs.is_empty() {
            if let Some(last_processed_input) = &client_info.last_processed_input {
                // if no new inputs, use their last known input
                for (char_state, char_consts, mut transform) in characters.iter_mut() {
                    if char_state.owner_client_id != client_info.client_id {
                        continue;
                    }
                    let speed = char_consts.move_speed * fixed_time.delta_seconds();
                    let wish_dir = compute_wish_dir(&last_processed_input);
                    transform.translation += wish_dir * speed;
                }
            }
        } else {
            let delta_chopped = fixed_time.delta_seconds() / client_info.inputs.len().max(1) as f32;
            for input in client_info.inputs.iter() {
                if let Some(last_processed_input) = &client_info.last_processed_input {
                    if input.id <= last_processed_input.id {
                        continue;
                    }
                }

                let mut last_processed_input = None;
                let wish_dir = compute_wish_dir(&input);

                for (char_state, char_consts, mut transform) in characters.iter_mut() {
                    if char_state.owner_client_id != client_info.client_id {
                        continue;
                    }

                    let speed = char_consts.move_speed * delta_chopped;
                    transform.translation += wish_dir * speed;
                    last_processed_input = Some(input.clone());
                }

                // acknowledge the input
                if let Some(last_processed_input) = last_processed_input {
                    client_info.last_processed_input = Some(last_processed_input.clone());
                }
            }

            client_info.inputs.clear();
        }
    }
}
