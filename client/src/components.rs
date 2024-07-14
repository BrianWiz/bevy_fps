use shared::bevy::prelude::*;

#[derive(Component)]
pub struct LocallyControlled;

#[derive(Component)]
pub struct ClientCorrection {
    pub offset: Vec3,
    pub velocity: Vec3,
}
