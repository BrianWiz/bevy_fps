use crate::{protocol::CharacterSnapshot, weapons::WeaponState};
use avian3d::{math::Quaternion, prelude::*};
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
        if let Some(server_velocity) = snapshot.velocity {
            self.velocity = server_velocity;
        }
        if let Some(server_position) = snapshot.position {
            existing_transform.translation = server_position;
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

    // accelerate
    velocity += accelerate(
        wish_dir,
        constants.move_speed,
        velocity.length(),
        constants.move_accel,
        delta_seconds,
    );

    // apply gravity
    if !state.is_grounded {
        velocity.y -= GRAVITY * delta_seconds;
    }

    let mut remaining_motion = velocity * delta_seconds;
    let radius = 0.5;
    let height = 1.0;
    let collider = Collider::cylinder(radius, height);
    let epsilon = 0.001;

    for _ in 0..4 {
        if let Some(hit) = spatial_query.cast_shape(
            &collider,
            transform.translation,
            Quaternion::IDENTITY,
            Dir3::new(remaining_motion.normalize()).unwrap_or(Dir3::Z),
            remaining_motion.length(),
            true,
            SpatialQueryFilter::default(),
        ) {
            let normal = hit.normal1;

            // Move to just before the collision point
            let move_distance = hit.time_of_impact;
            transform.translation += remaining_motion.normalize() * move_distance;

            // Check if we can step up
            if normal.y < constants.max_step_angle_degrees.to_radians().cos() {
                let step_height = constants.max_step_height;
                let step_up_position = transform.translation + Vec3::Y * step_height;

                // Check if there's space at the stepped-up position
                if spatial_query
                    .cast_shape(
                        &collider,
                        step_up_position,
                        Quaternion::IDENTITY,
                        Dir3::new(remaining_motion.normalize_or_zero()).unwrap_or(Dir3::Z),
                        remaining_motion.length(),
                        true,
                        SpatialQueryFilter::default(),
                    )
                    .is_none()
                {
                    // Cast down to find the actual step height
                    if let Some(down_hit) = spatial_query.cast_shape(
                        &collider,
                        step_up_position + remaining_motion,
                        Quaternion::IDENTITY,
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
                                remaining_motion + (Vec3::Y * (step_height + epsilon));
                            remaining_motion =
                                remaining_motion - normal * remaining_motion.dot(normal);
                            velocity = slide_velocity(velocity, normal);
                            continue;
                        }
                    }
                }
            }

            // prevent sticking
            transform.translation += normal * epsilon;

            // deflect velocity along the surface
            remaining_motion = remaining_motion - normal * remaining_motion.dot(normal);
            velocity = slide_velocity(velocity, normal);
        } else {
            // No collision, move the full remaining distance
            transform.translation += remaining_motion;
            break;
        }
    }

    if let Some(ground_info) = ground_check(spatial_query, transform, constants) {
        let ground_angle = ground_info.normal.y.acos().to_degrees();
        if ground_angle < constants.max_step_angle_degrees {
            state.is_grounded = true;

            // horizontal drag
            velocity.x = decelerate_component(
                velocity.x,
                velocity.length(),
                constants.move_drag,
                delta_seconds,
            );

            // horizontal drag
            velocity.z = decelerate_component(
                velocity.z,
                velocity.length(),
                constants.move_drag,
                delta_seconds,
            );
        }
    } else {
        state.is_grounded = false;
    }

    state.velocity = velocity;
}

fn slide_velocity(velocity: Vec3, normal: Vec3) -> Vec3 {
    velocity - normal * velocity.dot(normal)
}

fn ground_check(
    spatial_query: &SpatialQuery,
    transform: &Transform,
    constants: &CharacterConstants,
) -> Option<GroundInfo> {
    let height = 1.0;
    let collider = Collider::cylinder(0.5, constants.max_ground_distance);
    let bottom_position = transform.translation - Vec3::Y * (height / 2.0);

    if let Some(hit) = spatial_query.cast_shape(
        &collider,
        bottom_position,
        Quaternion::IDENTITY,
        Dir3::new(Vec3::NEG_Y).unwrap_or(Dir3::Z),
        constants.max_ground_distance,
        true,
        SpatialQueryFilter::default(),
    ) {
        return Some(GroundInfo {
            normal: hit.normal1,
        });
    }
    None
}

struct GroundInfo {
    normal: Vec3,
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

fn decelerate_component(component: f32, current_speed: f32, drag: f32, delta_seconds: f32) -> f32 {
    let mut new_speed;
    let mut drop = 0.0;

    drop += current_speed * drag * delta_seconds;

    new_speed = current_speed - drop;
    if new_speed < 0.0 {
        new_speed = 0.0;
    }

    if new_speed != 0.0 {
        new_speed /= current_speed;
    }

    component * new_speed
}
