use crate::{protocol::CharacterSnapshot, weapons::WeaponState};
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_quinnet::shared::ClientId;

const GRAVITY: f32 = 9.81;

#[derive(Event)]
pub struct CharacterDespawnEvent {
    pub client_id: u64,
}

#[derive(Component)]
pub struct CharacterConstants {
    pub move_drag: f32,
    pub move_accel: f32,
    pub move_speed: f32,
    pub max_ground_distance: f32,
    pub max_step_height: f32,
    pub max_step_angle_degrees: f32,
}

#[derive(Component)]
pub struct CharacterState {
    pub owner_client_id: ClientId,
    pub velocity: Vec3,
    pub visuals_offset: Vec3,
    pub is_grounded: bool,
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
                is_grounded: false,
            },
            CharacterConstants {
                move_drag: 10.0,
                move_accel: 20.5,
                move_speed: 4.0,
                max_step_height: 0.3,
                max_ground_distance: 0.1,
                max_step_angle_degrees: 45.0,
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
    let radius = 0.5;
    let height = 1.0;
    let collider = Collider::cylinder(radius, height);
    let epsilon = 0.001;

    // Apply acceleration
    velocity += accelerate(
        wish_dir,
        constants.move_speed,
        velocity.dot(wish_dir),
        constants.move_accel,
        delta_seconds,
    );

    if state.is_grounded {
        velocity *= 1.0 - constants.move_drag * delta_seconds;
    } else {
        velocity.y -= GRAVITY * delta_seconds;
    }

    let mut remaining_time = delta_seconds;
    let max_iterations = 4;

    for _ in 0..max_iterations {
        if remaining_time < epsilon || velocity.length_squared() < epsilon * epsilon {
            break;
        }

        let move_delta = velocity * remaining_time;

        if let Some(hit) = spatial_query.cast_shape(
            &collider,
            transform.translation,
            transform.rotation,
            Dir3::new(move_delta.normalize_or_zero()).unwrap_or(Dir3::Z),
            move_delta.length(),
            true,
            SpatialQueryFilter::default(),
        ) {
            let normal = hit.normal1;
            let distance_before_hit = hit.time_of_impact * move_delta.length();
            let time_before_hit = hit.time_of_impact * remaining_time;

            // Check if we can step up
            if normal.y < constants.max_step_angle_degrees.to_radians().cos() {
                let step_height = constants.max_step_height;
                let step_up_position = transform.translation + Vec3::Y * step_height;

                // Check if there's space at the stepped-up position
                if spatial_query
                    .cast_shape(
                        &collider,
                        step_up_position,
                        transform.rotation,
                        Dir3::new(move_delta.normalize_or_zero()).unwrap_or(Dir3::Z),
                        move_delta.length(),
                        true,
                        SpatialQueryFilter::default(),
                    )
                    .is_none()
                {
                    // Cast down to find the actual step height
                    if let Some(down_hit) = spatial_query.cast_shape(
                        &collider,
                        step_up_position + move_delta,
                        transform.rotation,
                        Dir3::NEG_Y,
                        step_height + epsilon,
                        true,
                        SpatialQueryFilter::default(),
                    ) {
                        let step_height =
                            down_hit.point1.y - (transform.translation.y - height / 2.0);

                        if step_height <= constants.max_step_height {
                            // Step up
                            transform.translation +=
                                move_delta + (Vec3::Y * (step_height + epsilon));
                            velocity = slide_velocity(velocity, normal);
                            remaining_time -= time_before_hit;
                            continue;
                        }
                    }
                }
            }

            velocity = slide_velocity(velocity, normal);

            // Move to just before the collision point
            transform.translation += move_delta.normalize() * distance_before_hit;

            // Prevent sticking to walls
            transform.translation += normal * epsilon;

            remaining_time -= time_before_hit;
        } else {
            // No collision, move the full distance
            transform.translation += move_delta;
            break;
        }
    }

    let ground_info = ground_check(
        spatial_query,
        transform,
        &collider,
        constants.max_ground_distance,
    );

    if ground_info.is_some() {
        state.is_grounded = true;
    } else {
        state.is_grounded = false;
    }

    state.velocity = velocity;
}

fn slide_velocity(mut velocity: Vec3, normal: Vec3) -> Vec3 {
    // Blend reflection and velocity to create a slight bounce effect, reducing the "stickiness" of walls
    let reflection = velocity - 2.0 * velocity.dot(normal) * normal;
    let slide_factor = 0.75; // Adjust this value to control slidiness
    velocity = velocity.lerp(reflection, slide_factor);

    // Project velocity onto the plane of the wall
    velocity -= normal * velocity.dot(normal).max(0.0);
    velocity
}

fn ground_check(
    spatial_query: &SpatialQuery,
    transform: &Transform,
    collider: &Collider,
    max_ground_distance: f32,
) -> Option<GroundInfo> {
    let down_ray = -transform.up();
    if let Some(hit) = spatial_query.cast_shape(
        collider,
        transform.translation,
        transform.rotation,
        down_ray,
        max_ground_distance,
        true,
        SpatialQueryFilter::default(),
    ) {
        if !is_wall_normal(hit.normal1) {
            return Some(GroundInfo {
                normal: hit.normal1,
                distance: hit.time_of_impact * max_ground_distance,
            });
        }
    }

    None
}

struct GroundInfo {
    normal: Vec3,
    distance: f32,
}

fn is_wall_normal(normal: Vec3) -> bool {
    normal.y.abs() < 0.1
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
