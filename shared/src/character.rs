use crate::protocol::CharacterSnapshot;
use bevy::prelude::*;
use bevy_quinnet::shared::ClientId;
use bevy_rapier3d::prelude::*;

const GRAVITY: f32 = 3.71; // mars gravity

#[derive(Event)]
pub struct CharacterDespawnEvent {
    pub client_id: u64,
}

#[derive(Component)]
pub struct CharacterConstants {
    pub move_drag: f32,
    pub move_accel: f32,
    pub move_speed: f32,
}

#[derive(Component)]
pub struct CharacterState {
    pub owner_client_id: ClientId,
    pub velocity: Vec3,
    pub visuals_offset: Vec3,
}

impl CharacterState {
    pub fn is_locally_controlled(&self, local_peer_id: ClientId) -> bool {
        self.owner_client_id == local_peer_id
    }

    pub fn apply_snapshot(
        &mut self,
        snapshot: &CharacterSnapshot,
        existing_transform: &mut Transform,
    ) {
        if let Some(velocity) = snapshot.velocity {
            self.velocity = velocity;
        }
        if let Some(position) = snapshot.position {
            existing_transform.translation = position;
        }
    }
}

#[derive(Component)]
pub struct CharacterVisuals {
    pub belongs_to: Entity,
}

pub fn spawn_character(
    commands: &mut Commands,
    owner_peer_id: ClientId,
    position: &Vec3,
) -> Entity {
    commands
        .spawn((
            CharacterState {
                owner_client_id: owner_peer_id,
                velocity: Vec3::ZERO,
                visuals_offset: Vec3::ZERO,
            },
            CharacterConstants {
                move_drag: 5.9,
                move_accel: 15.5,
                move_speed: 5.0,
            },
            SpatialBundle {
                transform: Transform::from_translation(position.clone()),
                ..default()
            },
        ))
        .id()
}

pub fn move_character(
    wish_dir: Vec3,
    physics: &mut ResMut<RapierContext>,
    state: &mut CharacterState,
    transform: &mut Transform,
    constants: &CharacterConstants,
    delta_seconds: f32,
) {
    let mut velocity = state.velocity;

    // Decelerate the character
    velocity *= 1.0 - constants.move_drag * delta_seconds;

    // apply gravity
    velocity.y -= GRAVITY * delta_seconds;

    // Accelerate the character in the desired direction
    velocity += accelerate(
        wish_dir,
        constants.move_speed,
        velocity.dot(wish_dir),
        constants.move_accel,
        delta_seconds,
    );

    let mut remaining_time = delta_seconds;
    let radius = 0.5;
    let epsilon = 0.0001;
    let filter = QueryFilter {
        flags: QueryFilterFlags::EXCLUDE_SENSORS,
        ..default()
    };
    for _ in 0..4 {
        let collider = Collider::ball(radius);
        let ray_pos = transform.translation;
        let ray_rot = transform.rotation;
        let ray_dir = velocity.normalize_or_zero();

        let max_time_of_impact = velocity.length() * remaining_time;

        let options = ShapeCastOptions {
            max_time_of_impact,
            ..default()
        };

        if let Some((_, toi)) =
            physics.cast_shape(ray_pos, ray_rot, ray_dir, &collider, options, filter)
        {
            if let Some(details) = toi.details {
                let normal = details.normal1;

                // compute penetration depth
                let penetration_depth = toi.time_of_impact * max_time_of_impact;

                // deproject
                transform.translation += normal * (penetration_depth + (epsilon * remaining_time));

                // slide
                velocity -= normal * velocity.dot(normal) * (1.0 + epsilon);

                // reduce time
                remaining_time *= 1.0 - toi.time_of_impact;
            }
        } else {
            // No collision, move the full distance
            transform.translation += velocity * remaining_time;
            break;
        }
    }

    state.velocity = velocity;
}

fn accelerate(
    wish_direction: Vec3,
    wish_speed: f32,
    current_speed: f32,
    accel: f32,
    delta_seconds: f32,
) -> Vec3 {
    let add_speed = wish_speed - current_speed;

    if add_speed <= 0.0 {
        return Vec3::ZERO;
    }

    let mut accel_speed = accel * delta_seconds * wish_speed;
    if accel_speed > add_speed {
        accel_speed = add_speed;
    }

    wish_direction * accel_speed
}
