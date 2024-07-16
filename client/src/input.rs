use bevy::prelude::*;
use bevy_quinnet::client::QuinnetClient;
use shared::{compute_wish_dir, CharacterConstants, CharacterState, ClientMessage};

use crate::{net::is_locally_controlled, PlayerController, TICKRATE_HZ};

pub fn s_gather_movement_input(
    mut controller: ResMut<PlayerController>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    controller.latest_input.id = controller.next_input_id;
    controller.latest_input.move_forward = keyboard_input.pressed(KeyCode::KeyW);
    controller.latest_input.move_backward = keyboard_input.pressed(KeyCode::KeyS);
    controller.latest_input.move_left = keyboard_input.pressed(KeyCode::KeyA);
    controller.latest_input.move_right = keyboard_input.pressed(KeyCode::KeyD);
    controller.next_input_id += 1;
}

pub fn s_consume_input(
    mut controller: ResMut<PlayerController>,
    mut character: Query<(&CharacterState, &CharacterConstants, &mut Transform)>,
    fixed_time: Res<Time<Fixed>>,
) {
    for (char_state, char_consts, mut transform) in character.iter_mut() {
        if is_locally_controlled(char_state, controller.client_id) {
            let wish_dir = compute_wish_dir(&controller.latest_input);
            let speed = char_consts.move_speed as f64 * fixed_time.delta_seconds_f64();
            let movement = wish_dir * speed as f32;
            transform.translation += movement;
            controller.latest_input.final_position = transform.translation;
            if !controller.is_replaying {
                // info!(
                //     "Fixed Time: {} | {}",
                //     fixed_time.delta_seconds(),
                //     1.0 / TICKRATE_HZ as f32
                // );
            }
        }
    }
}

pub fn s_send_input(mut client: ResMut<QuinnetClient>, mut controller: ResMut<PlayerController>) {
    // send the input to the server
    if let Err(err) = client
        .connection_mut()
        .send_message(ClientMessage::PlayerInput(controller.latest_input.clone()))
    {
        println!("Error sending message: {:?}", err);
    }

    // cache the input
    let input = controller.latest_input.clone();
    controller.input_history.push(input);

    // retain the last 3 seconds of inputs
    let oldest_input_id = controller.next_input_id.saturating_sub(TICKRATE_HZ * 3);
    controller
        .input_history
        .retain(|input| input.id >= oldest_input_id);
}
