use crate::{
    character::CharacterVisuals,
    net::{is_locally_controlled, PlayerController},
};
use avian3d::spatial_query::SpatialQuery;
use bevy::{input::mouse::MouseMotion, prelude::*};
use shared::{
    character::{move_character, CharacterConstants, CharacterState},
    input::compute_wish_dir,
};

const MOUSE_SENS: f32 = 0.1;

pub fn s_move_camera(
    mut camera: Query<(&Camera, &mut Transform)>,
    character: Query<&Transform, (With<CharacterVisuals>, Without<Camera>)>,
) {
    for (_, mut cam_xform) in camera.iter_mut() {
        for vis_xform in character.iter() {
            cam_xform.translation = vis_xform.translation + Vec3::new(0.0, 0.25, 0.0);
        }
    }
}

pub fn s_gather_look_input(
    mut controller: ResMut<PlayerController>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut camera: Query<&mut Transform, With<Camera>>,
    mut character: Query<(&CharacterState, &mut Transform), Without<Camera>>,
) {
    for (char_state, mut char_xform) in character.iter_mut() {
        if is_locally_controlled(char_state, controller.client_id) {
            for event in mouse_motion.read() {
                let mut yaw = char_xform.rotation.to_euler(EulerRot::YXZ).0;
                yaw -= event.delta.x.to_radians() * MOUSE_SENS;

                let mut pitch = 0.0;
                for mut cam_xform in camera.iter_mut() {
                    pitch = cam_xform.rotation.to_euler(EulerRot::YXZ).1;
                    pitch = (pitch - event.delta.y.to_radians() * MOUSE_SENS)
                        .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
                    cam_xform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
                }

                char_xform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, 0.0, 0.0);

                controller.latest_input.yaw = yaw;
                controller.latest_input.pitch = pitch;
            }
        }
    }
}

pub fn s_consume_look_input(
    controller: ResMut<PlayerController>,
    mut camera: Query<(&Camera, &mut Transform)>,
    mut character: Query<(&CharacterState, &mut Transform), Without<Camera>>,
) {
    for (char_state, mut char_xform) in character.iter_mut() {
        if is_locally_controlled(char_state, controller.client_id) {
            for (_, mut cam_xform) in camera.iter_mut() {
                cam_xform.rotation = Quat::from_euler(
                    EulerRot::YXZ,
                    controller.latest_input.yaw,
                    controller.latest_input.pitch,
                    0.0,
                );
            }
            char_xform.rotation =
                Quat::from_euler(EulerRot::YXZ, controller.latest_input.yaw, 0.0, 0.0);
        }
    }
}

pub fn s_gather_movement_input(
    mut controller: ResMut<PlayerController>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    controller.latest_input.id = controller.next_input_id;
    controller.latest_input.move_forward = keyboard_input.pressed(KeyCode::KeyW);
    controller.latest_input.move_backward = keyboard_input.pressed(KeyCode::KeyS);
    controller.latest_input.move_left = keyboard_input.pressed(KeyCode::KeyA);
    controller.latest_input.move_right = keyboard_input.pressed(KeyCode::KeyD);
    controller.latest_input.jump = keyboard_input.pressed(KeyCode::Space);
    controller.next_input_id += 1;
}

pub fn s_consume_move_input(
    spatial_query: SpatialQuery,
    mut controller: ResMut<PlayerController>,
    mut character: Query<(&mut CharacterState, &CharacterConstants, &mut Transform)>,
    fixed_time: Res<Time<Fixed>>,
) {
    for (mut char_state, char_consts, mut transform) in character.iter_mut() {
        if is_locally_controlled(&mut char_state, controller.client_id) {
            move_character(
                compute_wish_dir(&controller.latest_input),
                controller.latest_input.jump,
                &spatial_query,
                &mut char_state,
                &mut transform,
                &char_consts,
                fixed_time.delta_seconds(),
            );
            controller.latest_input.final_position = transform.translation;
        }
    }
}
