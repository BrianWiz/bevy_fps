use crate::snapshot::CharacterSnapshot;
use avian3d::{
    collision::Collider,
    math::Quaternion,
    spatial_query::{SpatialQuery, SpatialQueryFilter},
};
use bevy::prelude::*;

const GRAVITY: f32 = 9.81;

#[derive(Component)]
pub struct CharacterConstants {
    pub move_drag: f32,
    pub move_accel: f32,
    pub move_speed: f32,
    pub air_drag: f32,
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
    pub was_grounded: bool,
    pub is_grounded: bool,
}

struct GroundInfo {
    normal: Vec3,
}

pub fn spawn_character(commands: &mut Commands, char_snap: &CharacterSnapshot) -> Entity {
    commands
        .spawn((
            CharacterState {
                owner_client_id: char_snap.owner_client_id,
                velocity: char_snap.velocity.unwrap_or(Vec3::ZERO),
                was_grounded: false,
                is_grounded: false,
            },
            CharacterConstants {
                move_drag: 20.0,
                move_speed: 4.0,
                move_accel: 30.0,
                air_drag: 0.1,
                air_speed: 4.0,
                air_accel: 5.0,
                jump_strength: 2.2,
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

pub fn move_character(
    wish_dir: Vec3,
    jump: bool,
    spatial_query: &SpatialQuery,
    state: &mut CharacterState,
    transform: &mut Transform,
    constants: &CharacterConstants,
    delta_seconds: f32,
) {
    let mut velocity = state.velocity;

    state.was_grounded = state.is_grounded;
    state.is_grounded = ground_check(spatial_query, transform, constants).is_some();

    if !state.is_grounded {
        velocity = apply_air_drag(velocity, velocity.length());
        velocity += accelerate(
            wish_dir,
            constants.air_speed,
            velocity.length(),
            constants.air_accel,
            delta_seconds,
        );

        let mut end_velocity = velocity.y;
        end_velocity -= GRAVITY * delta_seconds;
        velocity.y = (velocity.y + end_velocity) * 0.5;
    } else {
        velocity = apply_ground_drag(
            velocity,
            velocity.length(),
            constants.move_drag,
            delta_seconds,
        );

        velocity += accelerate(
            wish_dir,
            constants.move_speed,
            velocity.dot(wish_dir),
            constants.move_accel,
            delta_seconds,
        );

        if jump {
            velocity.y = constants.jump_strength;
            state.is_grounded = false;
        }
    }

    // Move and handle collisions
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
            Dir3::new(remaining_motion.normalize_or_zero()).unwrap_or(Dir3::Z),
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

            // prevents sticking
            transform.translation += normal * epsilon;

            // deflect velocity along the surface
            remaining_motion = remaining_motion - normal * remaining_motion.dot(normal);
            velocity = slide_velocity(velocity, normal);
        } else {
            // no collision, move the full distance
            transform.translation += remaining_motion;
            break;
        }
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
        let ground_angle = hit.normal1.y.acos().to_degrees();
        if ground_angle < constants.max_step_angle_degrees {
            return Some(GroundInfo {
                normal: hit.normal1,
            });
        }
    }
    None
}

fn apply_ground_drag(velocity: Vec3, current_speed: f32, drag: f32, delta_seconds: f32) -> Vec3 {
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

    velocity * new_speed
}

fn apply_air_drag(velocity: Vec3, current_speed: f32) -> Vec3 {
    let mut new_speed;

    new_speed = current_speed;
    if new_speed < 0.0 {
        new_speed = 0.0;
    }

    if new_speed != 0.0 {
        new_speed /= current_speed;
    }

    velocity * new_speed
}

fn accelerate(
    wish_dir: Vec3,
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

    wish_dir * accel_speed
}
