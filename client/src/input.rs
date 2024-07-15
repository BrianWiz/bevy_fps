use shared::bevy::input::mouse::MouseMotion;
use shared::bevy::prelude::*;
use shared::character::*;
use shared::protocol::*;

use crate::components::LocallyControlled;

#[derive(Default, Resource)]
pub struct PlayerInputController {
    pub latest_input: PlayerInput,
    pub latest_acked_input: Option<PlayerInput>,
    pub input_history: Vec<PlayerInput>,
    pub next_input_id: u32,
}

impl PlayerInputController {
    pub fn inputs_after(&self, since_input_id: u32) -> Vec<PlayerInput> {
        self.input_history
            .iter()
            .filter(|input| input.id > since_input_id)
            .cloned()
            .collect::<Vec<PlayerInput>>()
    }

    pub fn get_input(&self, input_id: u32) -> Option<&PlayerInput> {
        self.input_history.iter().find(|input| input.id == input_id)
    }

    pub fn retain_inputs_after(&mut self, input_id: u32) {
        self.input_history.retain(|input| input.id > input_id);
    }
}

pub fn update_rotation_system(
    mut mouse_motion: EventReader<MouseMotion>,
    mut visuals: Query<&mut Transform, (With<LocallyControlled>, With<CharacterVisuals>)>,
    mut controller: ResMut<PlayerInputController>,
) {
    if let Ok(mut xform) = visuals.get_single_mut() {
        for event in mouse_motion.read() {
            // Extract current yaw and pitch
            let (mut yaw, mut pitch, _) = xform.rotation.to_euler(EulerRot::YXZ);

            // Apply new rotations
            yaw -= event.delta.x.to_radians() * crate::MOUSE_SENISITIVITY;
            pitch = (pitch - event.delta.y.to_radians() * crate::MOUSE_SENISITIVITY)
                .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);

            // Reconstruct rotation with locked roll
            xform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);

            // Update controller input
            controller.latest_input.yaw = yaw;
            controller.latest_input.pitch = pitch;
        }
    }
}

pub fn update_movement_system(
    mut controller: ResMut<PlayerInputController>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    controller.latest_input.id = controller.next_input_id;
    controller.latest_input.move_forward = keyboard_input.pressed(KeyCode::KeyW);
    controller.latest_input.move_backward = keyboard_input.pressed(KeyCode::KeyS);
    controller.latest_input.move_left = keyboard_input.pressed(KeyCode::KeyA);
    controller.latest_input.move_right = keyboard_input.pressed(KeyCode::KeyD);
    controller.latest_input.jump = keyboard_input.pressed(KeyCode::Space);
    controller.latest_input.fire = mouse_button.pressed(MouseButton::Left);
    controller.next_input_id += 1;
}

pub fn update_history_system(mut controller: ResMut<PlayerInputController>) {
    let latest_input = controller.latest_input.clone();
    controller.input_history.push(latest_input);

    // retain only the last 2 seconds of input history
    let oldest_input_id = controller.next_input_id.saturating_sub(crate::TICKRATE * 2);
    controller.retain_inputs_after(oldest_input_id);
}
