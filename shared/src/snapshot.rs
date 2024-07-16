use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ServerSnapshot {
    pub tick: u64,
    pub acked_input_id: Option<u64>,
    pub character_snapshots: Vec<CharacterSnapshot>,
}

impl ServerSnapshot {
    pub fn diff(&self, old: &ServerSnapshot) -> ServerSnapshot {
        let mut character_snapshots = Vec::new();

        for new_char in &self.character_snapshots {
            let old_char = old
                .character_snapshots
                .iter()
                .find(|old_char| old_char.owner_client_id == new_char.owner_client_id);

            if let Some(old_char) = old_char {
                character_snapshots.push(new_char.diff(old_char));
            } else {
                character_snapshots.push(new_char.clone());
            }
        }

        ServerSnapshot {
            tick: self.tick,
            acked_input_id: self.acked_input_id,
            character_snapshots,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct CharacterSnapshot {
    pub owner_client_id: u64,
    pub position: Option<Vec3>,
    pub velocity: Option<Vec3>,
}

impl CharacterSnapshot {
    pub fn diff(&self, old: &Self) -> Self {
        Self {
            owner_client_id: self.owner_client_id,
            position: if self.position != old.position {
                self.position
            } else {
                None
            },
            velocity: if self.velocity != old.velocity {
                self.velocity
            } else {
                None
            },
        }
    }
}
