use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct PlayerInput {
    pub id: u64,
    pub server_tick: u64,
    pub move_forward: bool,
    pub move_backward: bool,
    pub move_left: bool,
    pub move_right: bool,
    pub jump: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub final_position: Vec3,
}

pub fn compute_wish_dir(input: &PlayerInput) -> Vec3 {
    //let rotation = Quat::from_euler(EulerRot::YXZ, input.yaw, input.pitch, 0.0);
    let rotation = Quat::from_rotation_y(input.yaw);
    let mut wish_dir = Vec3::ZERO;

    if input.move_forward {
        wish_dir += rotation * -Vec3::Z;
    }
    if input.move_backward {
        wish_dir += rotation * Vec3::Z;
    }
    if input.move_left {
        wish_dir += rotation * -Vec3::X;
    }
    if input.move_right {
        wish_dir += rotation * Vec3::X;
    }
    wish_dir.normalize_or_zero()
}
