use crate::protocol::CharacterSnapshot;

impl CharacterSnapshot {
    pub fn diff(&self, old: &CharacterSnapshot) -> CharacterSnapshot {
        CharacterSnapshot {
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

    pub fn apply_diff(&self, old: &CharacterSnapshot) -> CharacterSnapshot {
        CharacterSnapshot {
            owner_client_id: self.owner_client_id,
            position: self.position.or(old.position),
            velocity: self.velocity.or(old.velocity),
        }
    }
}