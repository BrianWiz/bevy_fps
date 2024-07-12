use crate::weapons::WeaponConfig;
use bevy::{prelude::*, utils::HashMap};

#[derive(Resource, Default)]
pub struct DataAssetHandles {
    pub weapon_configs: HashMap<String, Handle<WeaponConfig>>,
}
