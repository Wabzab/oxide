use bevy::prelude::*;
use noise::utils::{NoiseMapBuilder, PlaneMapBuilder};
use noise::{Fbm, Perlin};

#[derive(Component)]
struct Player;

#[derive(Resource)]
struct GameWorld {
    noise_generator: Fbm<Perlin>,
}

const PLAYER_SPEED: f32 = 100.0;
const CAMERA_DECAY_RATE: f32 = 5.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(GameWorld {
            noise_generator: Fbm::<Perlin>::new(0),
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, move_camera))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    world: Res<GameWorld>,
) {
    // Draw procedural map
    let noise_map = PlaneMapBuilder::<_, 3>::new(&world.noise_generator).build();
    let (grid_width, grid_height) = noise_map.size();
    let tile_size = 8_f32;
    let start_x = -(grid_width as f32) * tile_size / 2.0;
    let start_y = -(grid_height as f32) * tile_size / 2.0;
    for col_x in 0..grid_width {
        for col_y in 0..grid_height {
            let val = noise_map.get_value(col_x, col_y);
            let x = start_x + col_x as f32 * tile_size;
            let y = start_y + col_y as f32 * tile_size;
            commands.spawn((
                Sprite {
                    custom_size: Some(Vec2::new(tile_size, tile_size)),
                    color: Color::hsl((360.0 * val) as f32, 0.5, 0.5),
                    ..default()
                },
                Transform::from_translation(Vec3::new(x, y, 0.0)),
            ));
        }
    }

    // Spawn the cameraman
    commands.spawn(Camera2d);

    // Spawn the fool
    commands.spawn((
        Player,
        Mesh2d(meshes.add(Circle::new(10.0))),
        MeshMaterial2d(materials.add(Color::hsl(350.0, 0.75, 0.75))),
        Transform::from_translation(Vec3::ZERO),
    ));
}

// Moves the player based on input
fn move_player(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut transform: Single<&mut Transform, With<Player>>,
) {
    let mut direction = Vec2::ZERO;
    if input.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if input.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }
    if input.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if input.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }

    if direction != Vec2::ZERO {
        let velocity = direction.normalize() * PLAYER_SPEED * time.delta_secs();
        transform.translation.x += velocity.x;
        transform.translation.y += velocity.y;
    }
}

// Moves the camera by smoothly following the player
fn move_camera(
    mut camera: Single<&mut Transform, (With<Camera2d>, Without<Player>)>,
    player: Single<&Transform, (With<Player>, Without<Camera2d>)>,
    time: Res<Time>,
) {
    let Vec3 { x, y, .. } = player.translation;
    let direction = Vec3::new(x, y, camera.translation.z);

    camera
        .translation
        .smooth_nudge(&direction, CAMERA_DECAY_RATE, time.delta_secs());
}
