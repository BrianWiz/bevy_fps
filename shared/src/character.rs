use crate::{protocol::CharacterSnapshot, weapons::WeaponState};
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_quinnet::shared::ClientId;

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
            WeaponState {
                weapon_config_tag: "rocket_launcher".to_string(),
                next_fire_time: 0,
                ammo: 100,
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
    spatial_query: &SpatialQuery,
    state: &mut CharacterState,
    transform: &mut Transform,
    constants: &CharacterConstants,
    delta_seconds: f32,
) {
    let mut velocity = state.velocity;

    // drag
    velocity *= 1.0 - constants.move_drag * delta_seconds;

    // gravity
    velocity.y -= GRAVITY * delta_seconds;

    // acceleration
    velocity += accelerate(
        wish_dir,
        constants.move_speed,
        velocity.dot(wish_dir),
        constants.move_accel,
        delta_seconds,
    );

    let mut remaining_distance = velocity.length() * delta_seconds;
    let radius = 0.5;
    let epsilon = 0.001;
    let collider = Collider::sphere(radius);
    let ignore_origin_penetration = true;

    // we loop 4 times because we may move then collide, then slide, then collide again
    for _ in 0..4 {
        if remaining_distance < epsilon || velocity.length_squared() < epsilon * epsilon {
            break;
        }

        let velocity_dir = velocity.normalize_or_zero();

        if let Some(first_hit) = spatial_query.cast_shape(
            &collider,
            transform.translation,
            transform.rotation,
            Dir3::new(velocity_dir).unwrap_or(Dir3::Z),
            remaining_distance,
            ignore_origin_penetration,
            SpatialQueryFilter::default(),
        ) {
            // move to the point of impact
            let move_distance = (first_hit.time_of_impact - epsilon).max(0.0);
            transform.translation += velocity_dir * move_distance;

            // slide along the surface next move
            let normal = first_hit.normal1;
            velocity = velocity - normal * velocity.dot(normal);

            // prevents sticking
            transform.translation += normal * epsilon;

            // update remaining distance
            remaining_distance -= move_distance;
        } else {
            // no collision, move the full remaining distance
            transform.translation += velocity_dir * remaining_distance;
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
