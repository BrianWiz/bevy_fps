use bevy::prelude::*;
use bevy_quinnet::shared::ClientId;
use shared::{
    character::{spawn_character, CharacterState},
    snapshot::CharacterSnapshot,
};

use crate::net::{is_locally_controlled, PlayerController};

#[derive(Component)]
pub struct CharacterVisuals {
    pub belongs_to: Entity,
    pub correction_offset: Vec3,
    pub owner_client_id: ClientId,
}

pub fn spawn_visual_character(
    commands: &mut Commands,
    char_snap: &CharacterSnapshot,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) {
    let entity = spawn_character(commands, char_snap);
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cylinder {
                half_height: 0.5,
                radius: 0.5,
            }),
            material: materials.add(Color::srgb(0.8, 0.1, 0.6)),
            transform: Transform::from_translation(char_snap.position.unwrap_or(Vec3::ZERO)),
            ..default()
        },
        CharacterVisuals {
            belongs_to: entity,
            correction_offset: Vec3::ZERO,
            owner_client_id: char_snap.owner_client_id,
        },
    ));
}

const MAX_OFFSET_DISTANCE: f32 = 0.9;
const VISUALS_SMOOTH_SPEED: f32 = 10.0;
const OFFSET_CORRECTION_SPEED: f32 = 20.0;

pub fn s_smooth_character_visuals(
    controller: Res<PlayerController>,
    time: Res<Time>,
    fixed_time: Res<Time<Fixed>>,
    characters: Query<(&CharacterState, &Transform)>,
    mut visuals: Query<(&mut CharacterVisuals, &mut Transform), Without<CharacterState>>,
) {
    let delta_time = time.delta_seconds();
    let fixed_delta_time = fixed_time.delta_seconds();
    let time_diff = time.elapsed_seconds() - fixed_time.elapsed_seconds();
    let fraction = (time_diff / fixed_delta_time).clamp(0.0, 1.0);

    // Define gravity
    let gravity = Vec3::new(0.0, -9.81, 0.0); // Typical Earth gravity, adjust as needed

    for (mut vis, mut vis_xform) in visuals.iter_mut() {
        if let Ok((char_state, char_xform)) = characters.get(vis.belongs_to) {
            if is_locally_controlled(char_state, controller.client_id) {
                vis_xform.translation = char_xform.translation;

                // if vis.correction_offset.length() > MAX_OFFSET_DISTANCE {
                //     vis.correction_offset =
                //         vis.correction_offset.normalize_or_zero() * MAX_OFFSET_DISTANCE;
                // }

                // let extrapolated_velocity = if char_state.is_grounded {
                //     char_state.velocity
                // } else {
                //     char_state.velocity + gravity
                // };

                // let target_position =
                //     char_xform.translation + extrapolated_velocity * fixed_delta_time;
                // let target_position = target_position.lerp(char_xform.translation, fraction);

                // vis_xform.translation = move_towards(
                //     vis_xform.translation,
                //     target_position + vis.correction_offset,
                //     char_state.velocity.length() * delta_time,
                // );

                // vis.correction_offset = move_towards(
                //     vis.correction_offset,
                //     Vec3::ZERO,
                //     OFFSET_CORRECTION_SPEED * delta_time,
                // );
            }
        }
    }
}

fn move_towards(current: Vec3, target: Vec3, max_distance_delta: f32) -> Vec3 {
    let to_vector = target - current;
    let distance = to_vector.length();

    if distance <= max_distance_delta {
        target
    } else if distance > 0.0 {
        current + to_vector / distance * max_distance_delta
    } else {
        current
    }
}
