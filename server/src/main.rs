use avian3d::{collision::Collider, prelude::RigidBody, PhysicsPlugins};
use bevy::{log::LogPlugin, prelude::*};
use bevy_quinnet::server::QuinnetServerPlugin;
use net::ServerState;

mod net;

const TICKRATE_HZ: u64 = 64;

fn main() {
    App::new()
        .add_plugins((
            MinimalPlugins,
            LogPlugin::default(),
            PhysicsPlugins::default(),
            QuinnetServerPlugin::default(),
        ))
        //====================================================
        // startup systems
        //====================================================
        .add_systems(Startup, (s_setup, net::s_start_listening).chain())
        //====================================================
        // fixed systems
        //====================================================
        .insert_resource(Time::<Fixed>::from_hz(TICKRATE_HZ as f64))
        .add_systems(
            FixedUpdate,
            (
                net::s_client_disconnected_system,
                net::s_handle_client_messages,
                net::s_consume_inputs,
                net::s_send_snapshot,
            )
                .chain(),
        )
        //====================================================
        // resources
        //====================================================
        .init_resource::<SceneSpawner>()
        .init_resource::<ServerState>()
        .init_resource::<Assets<Mesh>>() // needed for avian3d
        .run();
}

fn s_setup(mut commands: Commands) {
    // floor
    commands.spawn((
        SpatialBundle {
            transform: Transform::from_translation(Vec3::new(0.0, -0.5, 0.0)),
            ..default()
        },
        Collider::cuboid(10.0, 1.0, 10.0),
        RigidBody::Static,
    ));

    // wall 1.
    commands.spawn((
        SpatialBundle {
            transform: Transform::from_translation(Vec3::new(-5.0, 2.0, 0.0)),
            ..default()
        },
        Collider::cuboid(1.0, 4.0, 10.0),
        RigidBody::Static,
    ));

    // wall 2.
    commands.spawn((
        SpatialBundle {
            transform: Transform::from_translation(Vec3::new(5.0, 2.0, 0.0)),
            ..default()
        },
        Collider::cuboid(1.0, 4.0, 10.0),
        RigidBody::Static,
    ));

    // wall 3.
    commands.spawn((
        SpatialBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 2.0, -5.0)),
            ..default()
        },
        Collider::cuboid(10.0, 4.0, 1.0),
        RigidBody::Static,
    ));

    // wall 4.
    commands.spawn((
        SpatialBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 2.0, 5.0)),
            ..default()
        },
        Collider::cuboid(10.0, 4.0, 1.0),
        RigidBody::Static,
    ));

    // pillar 1.
    commands.spawn((
        SpatialBundle {
            transform: Transform::from_translation(Vec3::new(-1.0, 2.0, -1.0)),
            ..default()
        },
        Collider::cylinder(0.5, 8.0),
        RigidBody::Static,
    ));

    let num_steps = 20; // Number of steps in the staircase
    let step_width = 2.0;
    let step_height = 0.1;
    let step_depth = 0.5;

    for i in 0..num_steps {
        let x = 0.0;
        let y = i as f32 * step_height;
        let z = i as f32 * step_depth;

        commands.spawn((
            SpatialBundle {
                transform: Transform::from_translation(Vec3::new(x, y, z)),
                ..default()
            },
            Collider::cuboid(step_width, step_height, step_depth),
            RigidBody::Static,
        ));
    }
}
