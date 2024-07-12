use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::resources::DataAssetHandles;

#[derive(Component)]
pub struct WeaponState {
    pub weapon_config_tag: String,
    pub next_fire_time: u64,
    pub ammo: u32,
}

impl WeaponState {
    pub fn can_fire(&self, time: &Time<Fixed>) -> bool {
        let current_time = time.elapsed().as_millis() as u64;
        current_time >= self.next_fire_time
    }

    pub fn on_fire(&mut self, time: &Time<Fixed>, weapon_config: &WeaponConfig) {
        let current_time = time.elapsed().as_millis() as u64;
        self.next_fire_time = current_time + weapon_config.fire_rate_ms as u64;
        self.ammo -= 1;
    }
}

#[derive(Serialize, Deserialize, Asset, TypePath, Clone, Debug)]
pub struct WeaponConfig {
    pub tag: String,
    pub name: String,
    pub fire_rate_ms: u32,
    pub damage: u32,
}

pub fn get_weapon_config<'a>(
    data_asset_handles: &Res<DataAssetHandles>,
    weapon_configs: &'a Res<Assets<WeaponConfig>>,
    tag: &str,
) -> Option<&'a WeaponConfig> {
    let handle = data_asset_handles.weapon_configs.get(tag)?;
    weapon_configs.get(handle)
}

#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct WeaponFiredProjectileEvent {
    pub owner_client_id: u64,
    pub weapon_config_tag: String,
    pub origin: Vec3,
    pub direction: Vec3,
}

#[derive(Event, Serialize, Deserialize, Clone, Debug)]
pub struct WeaponFiredHitscanEvent {
    pub owner_client_id: u64,
    pub weapon_config_tag: String,
    pub origin: Vec3,
    pub endpoint: Vec3,
}
