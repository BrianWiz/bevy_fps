use crate::events::ClientConnectedEvent;
use shared::{bevy::prelude::*, character::spawn_character};

pub fn handle_client_connected_system(
    mut commands: Commands,
    mut client_connected_events: EventReader<ClientConnectedEvent>,
) {
    for event in client_connected_events.read() {
        spawn_character(&mut commands, event.client_id, &Vec3::new(0.0, 2.0, 0.0));
    }
}
