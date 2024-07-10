use crate::protocol::*;

impl TickSnapshot {
    pub fn diff(&self, old: &TickSnapshot) -> TickSnapshot {
        let mut characters = Vec::new();

        for new_char in &self.characters {
            let old_char = old
                .characters
                .iter()
                .find(|old_char| old_char.owner_client_id == new_char.owner_client_id);

            if let Some(old_char) = old_char {
                characters.push(new_char.diff(old_char));
            } else {
                characters.push(new_char.clone());
            }
        }

        TickSnapshot {
            tick: self.tick,
            acked_input_id: self.acked_input_id,
            characters,
        }
    }
}
