use bevy::prelude::*;

#[derive(Component)]
struct Player;

const PLAYER_SPEED: f32 = 100.0;
const CAMERA_DECAY_RATE: f32 = 5.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, move_camera))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
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

fn move_camera(
    mut camera: Single<&mut Transform, (With<Camera2d>, Without<Player>)>,
    player: Single<&mut Transform, (With<Player>, Without<Camera2d>)>,
    time: Res<Time>,
) {
    let Vec3 { x, y, .. } = player.translation;
    let direction = Vec3::new(x, y, camera.translation.z);

    camera
        .translation
        .smooth_nudge(&direction, CAMERA_DECAY_RATE, time.delta_secs());
}
