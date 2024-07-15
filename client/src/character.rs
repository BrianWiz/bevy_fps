use crate::components::{ClientCorrection, LocallyControlled};
use crate::input::PlayerInputController;
use shared::avian3d::spatial_query::SpatialQuery;
use shared::bevy::prelude::*;
use shared::bevy_quinnet::shared::ClientId;
use shared::character::*;
use shared::utils::move_towards;

pub fn spawn_character(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    commands: &mut Commands,
    owner_peer_id: ClientId,
    position: &Vec3,
    local: bool,
) {
    let entity = shared::character::spawn_character(commands, owner_peer_id, position);
    if local {
        commands.entity(entity).insert(ClientCorrection {
            offset: Vec3::ZERO,
            velocity: Vec3::ZERO,
        });
        commands.entity(entity).insert(LocallyControlled);
    }

    let mut visuals = commands.spawn((
        CharacterVisuals {
            belongs_to: entity,
            landing_impact_timer: 0.0,
        },
        PbrBundle {
            mesh: meshes.add(Mesh::from(Cylinder {
                radius: 0.5,
                half_height: 0.5,
            })),
            transform: Transform::from_translation(position.clone()),
            material: materials.add(Color::srgb(0.5, 0.5, 1.0)),
            ..default()
        },
    ));

    if local {
        visuals.insert(LocallyControlled);
    }
}

pub fn move_system(
    fixed_time: Res<Time<Fixed>>,
    spatial_query: SpatialQuery,
    mut input_controller: ResMut<PlayerInputController>,
    mut characters: Query<
        (&mut CharacterState, &mut Transform, &CharacterConstants),
        With<LocallyControlled>,
    >,
) {
    for (mut char_state, mut char_xform, char_constants /*mut controller*/) in characters.iter_mut()
    {
        move_character(
            input_controller.latest_input.compute_wish_dir(),
            input_controller.latest_input.jump,
            &spatial_query,
            &mut char_state,
            &mut char_xform,
            char_constants,
            fixed_time.delta_seconds(),
        );
        input_controller.latest_input.final_position = char_xform.translation;
    }
}

const OFFSET_CORRECTION_SPEED: f32 = 20.0; // Increased for more immediate response
const CORRECTION_SPEED: f32 = 10.0; // Increased for more immediate response
const VERTICAL_CORRECTION_SPEED: f32 = 30.0; // Even faster vertical correction
const LANDING_IMPACT_DURATION: f32 = 0.5; // Duration of landing impact effect in seconds

pub fn update_locally_controlled_visuals_system(
    time: Res<Time>,
    fixed_time: Res<Time<Fixed>>,
    mut char_query: Query<
        (&CharacterState, &mut ClientCorrection, &Transform),
        With<LocallyControlled>,
    >,
    mut visuals_query: Query<
        (&mut Transform, &mut CharacterVisuals),
        (With<LocallyControlled>, Without<CharacterState>),
    >,
) {
    let delta_time = time.delta_seconds();

    for (mut visuals_transform, mut char_visuals) in visuals_query.iter_mut() {
        if let Ok((char_state, mut correction, char_transform)) =
            char_query.get_mut(char_visuals.belongs_to)
        {
            // Calculate offset
            let offset =
                char_transform.translation + correction.offset - visuals_transform.translation;

            // Apply correction
            let horizontal_correction = offset.xz() * CORRECTION_SPEED * delta_time;
            let vertical_correction = if char_visuals.landing_impact_timer > 0.0 {
                offset.y
            } else {
                offset.y * VERTICAL_CORRECTION_SPEED * delta_time
            };

            visuals_transform.translation.x += horizontal_correction.x;
            visuals_transform.translation.z += horizontal_correction.y;
            visuals_transform.translation.y += vertical_correction;

            // Update correction offset
            correction.offset = correction
                .offset
                .lerp(Vec3::ZERO, OFFSET_CORRECTION_SPEED * delta_time);
        }
    }
}
pub fn update_camera_system(
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    visuals: Query<
        (&GlobalTransform, &Transform),
        (
            With<CharacterVisuals>,
            With<LocallyControlled>,
            Without<Camera3d>,
        ),
    >,
) {
    if let Ok((visuals_global_transform, visuals_transform)) = visuals.get_single() {
        if let Ok(mut camera_transform) = camera_query.get_single_mut() {
            camera_transform.translation = visuals_global_transform.translation();
            camera_transform.rotation = visuals_transform.rotation;
        }
    }
}

pub fn despawn_system(
    characters: Query<(Entity, &CharacterState)>,
    visuals: Query<(Entity, &CharacterVisuals)>,
    mut commands: Commands,
    mut character_despawn_events: EventReader<CharacterDespawnEvent>,
) {
    for event in character_despawn_events.read() {
        for (entity, char_state) in characters.iter() {
            if char_state.owner_client_id == event.client_id {
                visuals
                    .iter()
                    .filter(|(_, visuals)| visuals.belongs_to == entity)
                    .for_each(|(entity, _)| {
                        commands.entity(entity).despawn_recursive();
                    });
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}
