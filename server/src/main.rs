use shared::avian3d::prelude::*;
use shared::bevy::app::ScheduleRunnerPlugin;
use shared::bevy::asset::LoadedFolder;
use shared::bevy::log::LogPlugin;
use shared::bevy::prelude::*;
use shared::bevy_common_assets::ron::RonAssetPlugin;
use shared::bevy_quinnet::server::QuinnetServerPlugin;
use shared::resources::DataAssetHandles;
use shared::weapons::WeaponConfig;
use std::time::Duration;

mod characters;
mod events;
mod gamemode;
mod net;

const TICKRATE: u32 = 64;

#[derive(Resource, Default, DerefMut, Deref)]
pub struct DataFolder(Handle<LoadedFolder>);

fn main() {
    App::new()
        .add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 200.0,
            ))),
            AssetPlugin::default(),
            HierarchyPlugin::default(), // needed by Avian
            RonAssetPlugin::<WeaponConfig>::new(&["weapon.ron"]),
            LogPlugin::default(),
            QuinnetServerPlugin::default(),
            PhysicsPlugins::default(),
        ))
        //====================================================
        // systems at startup
        //====================================================
        .add_systems(Startup, (setup, net::start_listening_system))
        //====================================================
        // systems updating at the fixed tickrate
        //====================================================
        .insert_resource(Time::<Fixed>::from_hz(TICKRATE as f64))
        .add_systems(
            FixedUpdate,
            (
                net::handle_received_messages_system,
                net::handle_server_events_system,
                net::handle_client_connected_system,
                net::handle_client_disconnected_system,
                net::handle_client_input_system,
                gamemode::handle_client_connected_system,
                characters::consume_input_system,
                characters::despawn_system,
                net::snapshot_system,
                net::data_load_system,
            )
                .chain(),
        )
        //====================================================
        // resources
        //====================================================
        .insert_resource(net::Application::default())
        .insert_resource(SceneSpawner::default())
        .insert_resource(Assets::<Mesh>::default()) // needed by Avian
        .insert_resource(DataFolder::default())
        .init_resource::<DataAssetHandles>()
        //====================================================
        // events
        //====================================================
        .add_event::<events::ClientConnectedEvent>()
        .add_event::<events::ClientDisconnectedEvent>()
        .add_event::<events::ClientInputEvent>()
        .add_event::<shared::character::CharacterDespawnEvent>()
        //====================================================
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut data_asset_handles: ResMut<DataAssetHandles>,
) {
    data_asset_handles.weapon_configs.insert(
        "rocket_launcher".into(),
        asset_server.load::<WeaponConfig>("data/rocket_launcher.weapon.ron"),
    );

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
            SpatialBundle {
                transform: Transform::from_translation(Vec3::new(x, y, z)),
                ..default()
            },
            Collider::cuboid(step_width, step_height, step_depth),
            RigidBody::Static,
        ));
    }
}
