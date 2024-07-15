use bevy::prelude::*;
use bevy_quinnet::shared::{
    channels::{ChannelId, ChannelType, ChannelsConfiguration},
    ClientId,
};
use serde::{Deserialize, Serialize};

use crate::weapons::WeaponConfig;

mod impl_character_snapshot;
mod impl_player_input;
mod impl_tick_snapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Connect { username: String },
    Disconnect {},
    ChatMessage(String),
    PlayerInput(PlayerInput),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    WeaponConfig(WeaponConfig),
    TickSnapshot(TickSnapshot),
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct PlayerInput {
    pub id: u32,
    pub server_tick: Option<u32>,
    pub move_forward: bool,
    pub move_backward: bool,
    pub move_left: bool,
    pub move_right: bool,
    pub move_up: bool,
    pub move_down: bool,
    pub jump: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub fire: bool,
    pub final_position: Vec3,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct CharacterSnapshot {
    pub owner_client_id: ClientId,
    pub position: Option<Vec3>,
    pub velocity: Option<Vec3>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TickSnapshot {
    pub tick: u32,
    pub acked_input_id: Option<u32>,
    pub characters: Vec<CharacterSnapshot>,
}

#[repr(u8)]
pub enum ClientChannels {
    Events,
    PlayerInputs,
}
impl Into<ChannelId> for ClientChannels {
    fn into(self) -> ChannelId {
        self as ChannelId
    }
}
impl ClientChannels {
    pub fn channels_configuration() -> ChannelsConfiguration {
        ChannelsConfiguration::from_types(vec![
            ChannelType::OrderedReliable,
            ChannelType::Unreliable,
        ])
        .unwrap()
    }
}

#[repr(u8)]
pub enum ServerChannels {
    SnapshotDiff,
    SnapshotFull,
    ImportantData,
}
impl Into<ChannelId> for ServerChannels {
    fn into(self) -> ChannelId {
        self as ChannelId
    }
}
impl ServerChannels {
    pub fn channels_configuration() -> ChannelsConfiguration {
        ChannelsConfiguration::from_types(vec![
            ChannelType::Unreliable,
            ChannelType::UnorderedReliable,
            ChannelType::UnorderedReliable,
        ])
        .unwrap()
    }
}
