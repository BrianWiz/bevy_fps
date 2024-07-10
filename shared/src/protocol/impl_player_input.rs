use crate::protocol::PlayerInput;
use bevy::prelude::*;

impl PlayerInput {
    pub fn compute_wish_dir(&self) -> Vec3 {
        let rotation = Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0);
        let mut wish_dir = Vec3::ZERO;

        if self.move_forward {
            wish_dir += rotation * -Vec3::Z;
        }
        if self.move_backward {
            wish_dir += rotation * Vec3::Z;
        }
        if self.move_left {
            wish_dir += rotation * -Vec3::X;
        }
        if self.move_right {
            wish_dir += rotation * Vec3::X;
        }
        if self.move_up {
            wish_dir += Vec3::Y;
        }
        if self.move_down {
            wish_dir -= Vec3::Y;
        }
        if wish_dir.length_squared() > 0.0 {
            wish_dir = wish_dir.normalize();
        }
        wish_dir
    }
}
