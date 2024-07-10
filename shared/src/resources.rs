use crate::weapons::WeaponConfig;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct DataAssetHandles {
    pub weapon_configs: Vec<Handle<WeaponConfig>>,
}
