use bevy::prelude::*;

pub fn move_towards(current: Vec3, target: Vec3, max_distance_delta: f32) -> Vec3 {
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
