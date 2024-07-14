use crate::components::LocallyControlled;
use crate::input::PlayerInputController;
use shared::avian3d::spatial_query::SpatialQuery;
use shared::bevy::prelude::*;
use shared::bevy_quinnet::shared::ClientId;
use shared::character::*;
use shared::utils::move_towards;

/// The speed at which visual offsets are corrected.
/// Higher values result in faster corrections but may appear less smooth.
/// Lower values provide smoother transitions but take longer to correct discrepancies.
const VISUALS_CORRECT_SPEED: f32 = 1.0;

/// Multiplier for the maximum distance the visual can move in a single frame.
/// Higher values allow for quicker visual updates, while lower values create smoother but potentially less responsive movement.
const VISUAL_MOVE_SPEED_MULTIPLIER: f32 = 1.5;

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
        commands.entity(entity).insert(LocallyControlled);
    }
    spawn_character_visuals(meshes, materials, commands, entity, position, local);
}

fn spawn_character_visuals(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    commands: &mut Commands,
    belongs_to: Entity,
    position: &Vec3,
    local: bool,
) {
    let mut cmd = commands.spawn((
        CharacterVisuals { belongs_to },
        PbrBundle {
            mesh: meshes.add(Mesh::from(Cylinder {
                radius: 0.5,
                half_height: 0.5,
            })),
            material: materials.add(Color::srgb(0.5, 0.5, 1.0)),
            transform: Transform::from_translation(position.clone()),
            ..default()
        },
    ));

    if local {
        cmd.insert(LocallyControlled);
    }
}

pub fn move_system(
    fixed_time: Res<Time<Fixed>>,
    mut input_controller: ResMut<PlayerInputController>,
    mut spatial_query: SpatialQuery,
    mut characters: Query<
        (&mut CharacterState, &mut Transform, &CharacterConstants),
        With<LocallyControlled>,
    >,
) {
    for (mut char_state, mut char_xform, char_constants /*mut controller*/) in characters.iter_mut()
    {
        move_character(
            input_controller.latest_input.compute_wish_dir(),
            &mut spatial_query,
            &mut char_state,
            &mut char_xform,
            char_constants,
            fixed_time.delta_seconds(),
        );
        input_controller.latest_input.final_position = char_xform.translation;
    }
}

pub fn update_locally_controlled_visuals_system(
    time: Res<Time>,
    fixed_time: Res<Time<Fixed>>,
    mut char_query: Query<(&mut CharacterState, &Transform), With<LocallyControlled>>,
    mut visuals_query: Query<
        (&mut Transform, &CharacterVisuals),
        (With<LocallyControlled>, Without<CharacterState>),
    >,
) {
    let delta_time = time.delta_seconds();
    let fixed_delta_time = fixed_time.delta_seconds();
    let time_diff = time.elapsed_seconds() - fixed_time.elapsed_seconds();
    let fraction = (time_diff / fixed_delta_time).clamp(0.0, 1.0);

    for (mut visuals_transform, char_visuals) in visuals_query.iter_mut() {
        if let Ok((mut char_state, char_transform)) = char_query.get_mut(char_visuals.belongs_to) {
            visuals_transform.translation = char_transform.translation;
            // // Extrapolate the character's position based on current velocity
            // let extrapolated_position =
            //     char_transform.translation + (char_state.velocity * fixed_delta_time);

            // // Interpolate between current and extrapolated position
            // let target_position = char_transform
            //     .translation
            //     .lerp(extrapolated_position, fraction);

            // // Apply the visual offset for smooth corrections
            // let corrected_position = target_position + (char_state.visuals_offset * fraction);

            // // Smoothly move the visual representation towards the corrected position
            // visuals_transform.translation = move_towards(
            //     visuals_transform.translation,
            //     corrected_position,
            //     char_state.velocity.length() * delta_time * VISUAL_MOVE_SPEED_MULTIPLIER,
            // );

            // // Gradually reduce the visual offset to prevent straying too far from the actual position
            // let offset_reduction_factor = VISUALS_CORRECT_SPEED * delta_time;
            // char_state.visuals_offset = char_state
            //     .visuals_offset
            //     .lerp(Vec3::ZERO, offset_reduction_factor);
        }
    }
}

pub fn update_visuals_system(
    time: Res<Time>,
    fixed_time: Res<Time<Fixed>>,
    char_state: Query<(&CharacterState, &Transform), Without<LocallyControlled>>,
    mut visuals: Query<
        (&mut Transform, &CharacterVisuals),
        (Without<CharacterState>, Without<LocallyControlled>),
    >,
) {
    // for (mut visuals_transform, char_visuals) in visuals.iter_mut() {
    //     if let Ok((char_state, char_transform)) = char_state.get(char_visuals.belongs_to) {
    //         let time_diff = time.elapsed_seconds() - fixed_time.elapsed_seconds();
    //         let fraction = time_diff / fixed_time.delta_seconds();
    //         let extrapolated_position =
    //             char_transform.translation + (char_state.velocity * fixed_time.delta_seconds());
    //         // this is where we are within the final extrapolated position and the actual position
    //         let target_position = char_transform
    //             .translation
    //             .lerp(extrapolated_position, fraction);
    //         visuals_transform.translation = target_position;
    //     }
    // }
}

pub fn update_camera_system(
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    character: Query<
        &Transform,
        (
            With<CharacterState>,
            With<LocallyControlled>,
            Without<Camera3d>,
        ),
    >,
    visuals: Query<
        &Transform,
        (
            With<CharacterVisuals>,
            With<LocallyControlled>,
            Without<Camera3d>,
            Without<CharacterState>,
        ),
    >,
) {
    if let Ok(visuals_global_transform) = visuals.get_single() {
        if let Ok(mut camera_transform) = camera_query.get_single_mut() {
            if let Ok(character_transform) = character.get_single() {
                camera_transform.translation = visuals_global_transform.translation;
                camera_transform.rotation = character_transform.rotation;
            }
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
