use bevy::prelude::*;

use crate::modules::collision::{Collidable, ContactDamage, DespawnOnHit, Shape};

const PLAYER_SPEED: f32 = 200.0;
const CAMERA_DECAY_RATE: f32 = 5.0;
pub const PLAYER_RADIUS: f32 = 12.0;
const SMASH_RADIUS: f32 = 48.0;
const SMASH_DAMAGE: f32 = 1.0;

#[derive(Clone, Copy)]
enum Binding {
    Key(KeyCode),
    Mouse(MouseButton),
}

impl Binding {
    fn pressed(self, keys: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>) -> bool {
        match self {
            Binding::Key(k) => keys.pressed(k),
            Binding::Mouse(m) => mouse.pressed(m),
        }
    }
    fn just_pressed(self, keys: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>) -> bool {
        match self {
            Binding::Key(k) => keys.just_pressed(k),
            Binding::Mouse(m) => mouse.just_pressed(m),
        }
    }
}

fn any_pressed(
    bindings: impl IntoIterator<Item = Binding>,
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
) -> bool {
    bindings.into_iter().any(|b| b.pressed(keys, mouse))
}

fn all_pressed(
    bindings: impl IntoIterator<Item = Binding>,
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
) -> bool {
    bindings.into_iter().all(|b| b.pressed(keys, mouse))
}

#[derive(Resource)]
pub struct PlayerControls {
    move_left: Binding,
    move_right: Binding,
    move_up: Binding,
    move_down: Binding,
    attack: Binding,
    mine: Binding,
}

impl PlayerControls {
    fn move_keys(&self) -> [Binding; 4] {
        [
            self.move_left,
            self.move_right,
            self.move_up,
            self.move_down,
        ]
    }

    fn is_moving(&self, keys: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>) -> bool {
        any_pressed(self.move_keys(), keys, mouse)
    }
}

impl From<KeyCode> for Binding {
    fn from(k: KeyCode) -> Self {
        Binding::Key(k)
    }
}
impl From<MouseButton> for Binding {
    fn from(m: MouseButton) -> Self {
        Binding::Mouse(m)
    }
}

#[derive(Component)]
pub struct Player;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlayerControls {
            move_left: KeyCode::KeyA.into(),
            move_right: KeyCode::KeyD.into(),
            move_up: KeyCode::KeyW.into(),
            move_down: KeyCode::KeyS.into(),
            attack: MouseButton::Left.into(),
            mine: MouseButton::Right.into(),
        });
        app.add_systems(Startup, setup);
        app.add_systems(Update, (player_controller, move_camera));
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

// Handles user input for controller the player.
fn player_controller(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    controls: Res<PlayerControls>,
    time: Res<Time>,
    mut transform: Single<&mut Transform, With<Player>>,
    mut commands: Commands,
) {
    if controls.is_moving(&keys, &mouse) {
        move_player(&keys, &mouse, &controls, &time, &mut transform);
    }

    if controls.mine.just_pressed(&keys, &mouse) {
        player_smash(&transform, &mut commands);
    }
}

// Moves the player based on input
fn move_player(
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    controls: &PlayerControls,
    time: &Time,
    transform: &mut Transform,
) {
    let mut direction = Vec2::ZERO;
    if controls.move_left.pressed(keys, mouse) {
        direction.x -= 1.0;
    }
    if controls.move_right.pressed(keys, mouse) {
        direction.x += 1.0;
    }
    if controls.move_up.pressed(keys, mouse) {
        direction.y += 1.0;
    }
    if controls.move_down.pressed(keys, mouse) {
        direction.y -= 1.0;
    }

    if direction != Vec2::ZERO {
        let velocity = direction.normalize() * PLAYER_SPEED * time.delta_secs();
        transform.translation.x += velocity.x;
        transform.translation.y += velocity.y;
    }
}

// Spawns a one-shot damage pulse at the player on Space. DespawnOnHit makes the damage system
// consume it after a single pass, so it deals contact damage to overlapping breakables once.
fn player_smash(transform: &Transform, commands: &mut Commands) {
    commands.spawn((
        ContactDamage {
            amount: SMASH_DAMAGE,
        },
        DespawnOnHit,
        Collidable {
            shape: Shape::circle(SMASH_RADIUS),
        },
        Transform::from_translation(transform.translation),
    ));
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
