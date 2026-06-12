use bevy::prelude::*;

use crate::modules::collision::{Collidable, ContactDamage, DespawnOnHit, Shape};

const PLAYER_SPEED: f32 = 200.0;
const CAMERA_DECAY_RATE: f32 = 5.0;
pub const PLAYER_RADIUS: f32 = 12.0;
const SMASH_RADIUS: f32 = 48.0;
const SMASH_DAMAGE: f32 = 1.0;

#[derive(Component)]
pub struct Player;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(Update, (move_player, move_camera, player_smash));
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Spawn the fool
    commands.spawn((
        Player,
        Mesh2d(meshes.add(Circle::new(PLAYER_RADIUS))),
        MeshMaterial2d(materials.add(Color::hsl(350.0, 0.75, 0.75))),
        Transform::from_translation(Vec3::ZERO),
        Collidable {
            shape: Shape::circle(PLAYER_RADIUS),
        },
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

// Spawns a one-shot damage pulse at the player on Space. DespawnOnHit makes the damage system
// consume it after a single pass, so it deals contact damage to overlapping breakables once.
fn player_smash(
    input: Res<ButtonInput<KeyCode>>,
    player: Single<&Transform, With<Player>>,
    mut commands: Commands,
) {
    if input.just_pressed(KeyCode::Space) {
        commands.spawn((
            ContactDamage {
                amount: SMASH_DAMAGE,
            },
            DespawnOnHit,
            Collidable {
                shape: Shape::circle(SMASH_RADIUS),
            },
            Transform::from_translation(player.translation),
        ));
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
