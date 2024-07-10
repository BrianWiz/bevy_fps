use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Asset, TypePath, Clone, Debug)]
pub struct WeaponConfig {
    pub tag: String,
    pub name: String,
    pub fire_rate_ms: u32,
    pub damage: u32,
}
