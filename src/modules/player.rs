use bevy::prelude::*;

use crate::modules::collision::Collidable;

const PLAYER_SPEED: f32 = 200.0;
const CAMERA_DECAY_RATE: f32 = 5.0;
pub const PLAYER_SIZE: f32 = 24.0;

#[derive(Component)]
pub struct Player;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(Update, (move_player, move_camera));
    }
}

fn setup(mut commands: Commands) {
    // Spawn the fool
    commands.spawn((
        Player,
        Sprite {
            custom_size: Some(Vec2::new(PLAYER_SIZE, PLAYER_SIZE)),
            color: Color::hsl(350.0, 0.75, 0.75),
            ..default()
        },
        Transform::from_translation(Vec3::ZERO),
        Collidable { size: PLAYER_SIZE },
    ));

    // Spawn the cameraman
    commands.spawn(Camera2d);
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
