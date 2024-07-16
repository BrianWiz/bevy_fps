use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use bevy::winit::WinitSettings;
use bevy_quinnet::client::client_connected;
use bevy_quinnet::client::QuinnetClientPlugin;
use net::NetworkMetrics;
use net::PlayerController;
use std::time::Duration;

mod character;
mod input;
mod net;

const TICKRATE_HZ: u64 = 64;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            PhysicsDebugPlugin::default(),
            QuinnetClientPlugin::default(),
        ))
        .insert_resource(WinitSettings {
            focused_mode: bevy::winit::UpdateMode::Continuous,
            unfocused_mode: bevy::winit::UpdateMode::Continuous,
        })
        //====================================================
        // startup systems
        //====================================================
        .add_systems(Startup, (s_setup, net::s_start_connection).chain())
        //====================================================
        // systems that run every tick
        //====================================================
        .add_systems(
            Update,
            (
                input::s_gather_look_input,
                input::s_consume_look_input,
                character::s_smooth_character_visuals,
                input::s_move_camera,
                net::s_log_network_metrics.run_if(on_timer(Duration::from_secs(5))),
            )
                .chain(),
        )
        //====================================================
        // fixed systems
        //====================================================
        .insert_resource(Time::<Fixed>::from_hz(TICKRATE_HZ as f64))
        .add_systems(
            FixedUpdate,
            (
                net::s_handle_server_messages,
                net::s_consume_snapshot_buffer,
                input::s_gather_movement_input,
                input::s_consume_move_input,
                net::s_send_input,
            )
                .chain()
                .run_if(client_connected),
        )
        //====================================================
        // resources
        //====================================================
        .init_resource::<PlayerController>()
        .init_resource::<NetworkMetrics>()
        .run();
}

fn s_setup(
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
