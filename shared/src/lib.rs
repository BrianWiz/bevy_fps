use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub enum ClientMessage {
    PlayerInput(PlayerInput),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ServerMessage {
    ServerSnapshot(ServerSnapshot),
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct PlayerInput {
    pub id: u64,
    pub server_tick: u64,
    pub move_forward: bool,
    pub move_backward: bool,
    pub move_left: bool,
    pub move_right: bool,
    pub final_position: Vec3,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ServerSnapshot {
    pub tick: u64,
    pub acked_input_id: Option<u64>,
    pub character_snapshots: Vec<CharacterSnapshot>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct CharacterSnapshot {
    pub owner_client_id: u64,
    pub position: Option<Vec3>,
    pub velocity: Option<Vec3>,
}

#[derive(Component)]
pub struct CharacterConstants {
    pub move_drag: f32,
    pub move_accel: f32,
    pub move_speed: f32,
    pub air_speed: f32,
    pub air_accel: f32,
    pub jump_strength: f32,
    pub max_ground_distance: f32,
    pub max_step_height: f32,
    pub max_step_angle_degrees: f32,
}

#[derive(Component)]
pub struct CharacterState {
    pub owner_client_id: u64,
    pub velocity: Vec3,
}

pub fn spawn_character(commands: &mut Commands, char_snap: &CharacterSnapshot) -> Entity {
    commands
        .spawn((
            CharacterState {
                owner_client_id: char_snap.owner_client_id,
                velocity: char_snap.velocity.unwrap_or(Vec3::ZERO),
            },
            CharacterConstants {
                move_drag: 30.0,
                move_speed: 4.0,
                move_accel: 30.0,
                air_speed: 4.0,
                air_accel: 5.0,
                jump_strength: 3.0,
                max_step_height: 0.3,
                max_ground_distance: 0.1,
                max_step_angle_degrees: 45.0,
            },
            Transform {
                translation: char_snap.position.unwrap_or(Vec3::ZERO),
                ..Default::default()
            },
        ))
        .id()
}

pub fn compute_wish_dir(input: &PlayerInput) -> Vec3 {
    let mut wish_dir = Vec3::ZERO;
    if input.move_forward {
        wish_dir += Vec3::Z;
    }
    if input.move_backward {
        wish_dir -= Vec3::Z;
    }
    if input.move_right {
        wish_dir += Vec3::X;
    }
    if input.move_left {
        wish_dir -= Vec3::X;
    }
    wish_dir.normalize_or_zero()
}
