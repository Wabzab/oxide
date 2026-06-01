use crate::modules::collision::{Breakable, Collidable};
use bevy::prelude::*;
use noise::{
    Fbm, OpenSimplex,
    utils::{NoiseMapBuilder, PlaneMapBuilder},
};
use rand::prelude::*;

pub const TILE_SIZE: f32 = 32.0;

#[derive(Component)]
pub struct Tile;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands) {
    // Create seeded noise maps
    let seed = rand::rng().random();

    let mut cave_noise = Fbm::<OpenSimplex>::new(seed);
    cave_noise.frequency = 5.0;
    cave_noise.persistence = 0.45;
    let cave_noise_map = PlaneMapBuilder::<_, 3>::new(&cave_noise).build();

    let mut ore_noise = Fbm::<OpenSimplex>::new(seed + 1);
    ore_noise.frequency = 8.0;
    ore_noise.persistence = 0.5;
    let ore_noise_map = PlaneMapBuilder::<_, 3>::new(&ore_noise).build();

    // Spawn tiles
    let (grid_width, grid_height) = cave_noise_map.size();
    let start_x = -(grid_width as f32) * TILE_SIZE / 2.0;
    let start_y = -(grid_height as f32) * TILE_SIZE / 2.0;
    for col_x in 0..grid_width {
        for col_y in 0..grid_height {
            let x = start_x + col_x as f32 * TILE_SIZE;
            let y = start_y + col_y as f32 * TILE_SIZE;

            let is_cave_wall = cave_noise_map.get_value(col_x, col_y) > 0.0;
            if is_cave_wall {
                let is_ore_wall = ore_noise_map.get_value(col_x, col_y) > 0.2;
                if is_ore_wall {
                    spawn_ore_tile(&mut commands, Vec2::new(x, y));
                } else {
                    spawn_wall_tile(&mut commands, Vec2::new(x, y));
                }
            } else {
                spawn_floor_tile(&mut commands, Vec2::new(x, y));
            }
        }
    }
}

pub fn spawn_floor_tile(commands: &mut Commands, translation: Vec2) {
    commands.spawn((
        Tile,
        Transform::from_translation(translation.extend(-1.0)),
        Sprite {
            custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)),
            color: Color::hsl(250.0, 0.2, 0.15), // Dark blue
            ..default()
        },
    ));
}

pub fn spawn_wall_tile(commands: &mut Commands, translation: Vec2) {
    commands.spawn((
        Tile,
        Transform::from_translation(translation.extend(0.0)),
        Sprite {
            custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)),
            color: Color::hsl(165.0, 0.1, 0.35), // Light blue
            ..default()
        },
        Collidable { size: TILE_SIZE },
        Breakable { health: 1.0 },
    ));
}

pub fn spawn_ore_tile(commands: &mut Commands, translation: Vec2) {
    commands.spawn((
        Tile,
        Transform::from_translation(translation.extend(0.0)),
        Sprite {
            custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)),
            color: Color::hsl(45.0, 0.7, 0.25), // Golden
            ..default()
        },
        Collidable { size: TILE_SIZE },
        Breakable { health: 3.0 },
    ));
}
