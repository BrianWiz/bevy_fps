use input::PlayerInputController;
use shared::avian3d::prelude::*;
use shared::bevy::prelude::*;
use shared::bevy_quinnet::client::client_connected;
use shared::bevy_quinnet::client::QuinnetClientPlugin;
use shared::resources::DataAssetHandles;

mod character;
pub mod components;
mod input;
mod net;

pub const TICKRATE: u32 = 64;
pub const MOUSE_SENISITIVITY: f32 = 0.1;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            PhysicsDebugPlugin::default(),
            QuinnetClientPlugin::default(),
        ))
        //====================================================
        // systems at startup
        //====================================================
        .add_systems(Startup, (net::start_connection_system, setup_system))
        //====================================================
        // systems updating every tick
        //====================================================
        .add_systems(
            Update,
            (
                input::update_rotation_system,
                character::update_locally_controlled_visuals_system,
                character::update_visuals_system,
                character::update_camera_system,
            )
                .chain(),
        )
        .add_systems(PostUpdate, (net::on_app_exit_system,).chain())
        //====================================================
        // systems updating at the fixed tickrate
        //====================================================
        .insert_resource(Time::<Fixed>::from_hz(TICKRATE as f64))
        .add_systems(
            FixedUpdate,
            (
                net::handle_client_events_system,
                net::handle_received_messages_system.run_if(client_connected),
                input::update_movement_system,
                input::update_history_system,
                character::despawn_system,
                character::move_system,
                net::send_input_system.run_if(client_connected),
            )
                .chain(),
        )
        //====================================================
        // resources
        //====================================================
        .insert_resource(PlayerInputController::default())
        .init_resource::<DataAssetHandles>()
        //====================================================
        // assets
        //====================================================
        .init_asset::<shared::weapons::WeaponConfig>()
        //====================================================
        // events
        //====================================================
        .add_event::<shared::character::CharacterDespawnEvent>()
        .run();
}

fn setup_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 1.5, 5.0)),
        projection: Projection::Perspective(PerspectiveProjection {
            fov: 90.0_f32.to_radians(),
            ..default()
        }),
        ..default()
    });

    // floor
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(Cuboid {
                half_size: Vec3::new(5.0, 0.5, 5.0),
            })),
            material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
            transform: Transform::from_translation(Vec3::new(0.0, -0.5, 0.0)),
            ..default()
        },
        Collider::cuboid(10.0, 1.0, 10.0),
        RigidBody::Static,
    ));

    // wall 1.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(Cuboid {
                half_size: Vec3::new(0.5, 2.0, 5.0),
            })),
            material: materials.add(Color::srgb(0.5, 0.3, 0.3)),
            transform: Transform::from_translation(Vec3::new(-5.0, 2.0, 0.0)),
            ..default()
        },
        Collider::cuboid(1.0, 4.0, 10.0),
        RigidBody::Static,
    ));

    // wall 2.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(Cuboid {
                half_size: Vec3::new(0.5, 2.0, 5.0),
            })),
            material: materials.add(Color::srgb(0.3, 0.3, 0.5)),
            transform: Transform::from_translation(Vec3::new(5.0, 2.0, 0.0)),
            ..default()
        },
        Collider::cuboid(1.0, 4.0, 10.0),
        RigidBody::Static,
    ));

    // wall 3.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(Cuboid {
                half_size: Vec3::new(5.0, 2.0, 0.5),
            })),
            material: materials.add(Color::srgb(0.3, 0.5, 0.5)),
            transform: Transform::from_translation(Vec3::new(0.0, 2.0, -5.0)),
            ..default()
        },
        Collider::cuboid(10.0, 4.0, 1.0),
        RigidBody::Static,
    ));

    // wall 4.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(Cuboid {
                half_size: Vec3::new(5.0, 2.0, 0.5),
            })),
            material: materials.add(Color::srgb(0.5, 0.5, 0.3)),
            transform: Transform::from_translation(Vec3::new(0.0, 2.0, 5.0)),
            ..default()
        },
        Collider::cuboid(10.0, 4.0, 1.0),
        RigidBody::Static,
    ));

    // pillar 1.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(Cylinder {
                radius: 0.5,
                half_height: 4.0,
                ..Default::default()
            })),
            material: materials.add(Color::srgb(0.5, 0.5, 0.5)),
            transform: Transform::from_translation(Vec3::new(-1.0, 2.0, -1.0)),
            ..default()
        },
        Collider::cylinder(0.5, 8.0),
        RigidBody::Static,
    ));

    // small staircase
    let num_steps = 20; // Number of steps in the staircase
    let step_width = 2.0;
    let step_height = 0.1;
    let step_depth = 0.5;

    for i in 0..num_steps {
        let x = 0.0;
        let y = i as f32 * step_height;
        let z = i as f32 * step_depth;

        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(Cuboid {
                    half_size: Vec3::new(step_width / 2.0, step_height / 2.0, step_depth / 2.0),
                })),
                material: materials.add(Color::srgb(0.5, 0.5, 0.5)),
                transform: Transform::from_translation(Vec3::new(x, y, z)),
                ..default()
            },
            Collider::cuboid(step_width, step_height, step_depth),
            RigidBody::Static,
        ));
    }

    // light
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });
}
