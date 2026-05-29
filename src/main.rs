use bevy::prelude::*;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Tile;

#[derive(Component)]
struct Chunk {
    size: f32,
    pub tiles: Vec<Entity>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, check_chunks))
        .run();
}

fn setup(mut commands: Commands) {
    // Spawn the cameraman
    commands.spawn(Camera2d);

    // Spawn the fool
    commands.spawn((
        Player,
        Text2d::new("@"),
        TextFont {
            font_size: 12.0,
            font: default(),
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_translation(Vec3::ZERO),
    ));

    // Spawn the map's main chunks
    let width: i32 = 32;
    let height: i32 = 32;
    let size: f32 = 256.0;

    for x in 0..width {
        for y in 0..height {
            commands.spawn((
                Chunk {
                    size,
                    tiles: vec![],
                },
                Transform::from_translation(Vec3 {
                    x: (x as f32 - width as f32 / 2.0) * size,
                    y: (y as f32 - height as f32 / 2.0) * size,
                    z: 0.0, // Terrain layer?
                }),
            ));
        }
    }
}

fn move_player(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player_transform: Single<&mut Transform, With<Player>>,
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
        let speed = 150.0;
        let velocity = direction.normalize() * speed * time.delta_secs();
        player_transform.translation.x += velocity.x;
        player_transform.translation.y += velocity.y;
    }
}

fn check_chunks(
    mut commands: Commands,                       // for deleting/adding chunks
    mut meshes: ResMut<Assets<Mesh>>,             // for tile meshes
    mut materials: ResMut<Assets<ColorMaterial>>, // for tile colors
    mut query: Query<(&mut Chunk, &Transform)>,
    player_transform: Single<&Transform, With<Player>>,
) {
    let render_distance = 512.0;
    for (mut chunk, transform) in query.iter_mut() {
        let tile_size = 32.0;
        let tile_count = chunk.size / tile_size;
        let distance = (transform.translation - player_transform.translation).length();
        if chunk.tiles.len() == 0 && distance <= render_distance {
            for x in 0..tile_count as i32 {
                for y in 0..tile_count as i32 {
                    let tile = commands
                        .spawn((
                            Tile,
                            Transform::from_translation(
                                transform.translation
                                    + Vec3 {
                                        x: (x as f32 - tile_count / 2.0) * tile_size,
                                        y: (y as f32 - tile_count / 2.0) * tile_size,
                                        z: 0.0,
                                    },
                            ),
                            Mesh2d(meshes.add(Rectangle::new(tile_size, tile_size))),
                            MeshMaterial2d(materials.add(Color::hsl(25.0, 0.35, 0.5))),
                        ))
                        .id();
                    chunk.tiles.push(tile);
                }
            }
        }

        if chunk.tiles.len() != 0 && distance > render_distance {
            while chunk.tiles.len() > 0 {
                let tile = chunk.tiles.pop();
                commands.entity(tile.unwrap()).despawn();
            }
        }
    }
}
