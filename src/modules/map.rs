use crate::modules::collision::{Collidable, Death, Health, Shape};
use bevy::prelude::*;
use noise::{
    Fbm, OpenSimplex,
    utils::{NoiseMap, NoiseMapBuilder, PlaneMapBuilder},
};
use rand::prelude::*;

pub const TILE_SIZE: f32 = 32.0;
pub const MAP_WIDTH: usize = 100;
pub const MAP_HEIGHT: usize = 100;
pub const CARVE_RADIUS: usize = 4; // cells, half-extent of the square clearing at the center

// The two layers store one cell value each, addressed by index(x, y). Floor is the walkable
// ground that exists everywhere; Block is what sits on top of it (a wall, ore, or nothing).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Floor {
    #[default]
    None,
    Floor,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Block {
    #[default]
    None,
    Wall,
    Ore,
}

// Global source of truth for the level. The generation stages mutate this resource; the render
// step reads it to spawn entities; the death observer clears block cells as they break.
#[derive(Resource)]
pub struct TileMap {
    floor: Vec<Floor>,
    block: Vec<Block>,
}

impl Default for TileMap {
    fn default() -> Self {
        Self {
            floor: vec![Floor::None; MAP_WIDTH * MAP_HEIGHT],
            block: vec![Block::None; MAP_WIDTH * MAP_HEIGHT],
        }
    }
}

fn index(x: usize, y: usize) -> usize {
    y * MAP_WIDTH + x
}

impl TileMap {
    pub fn floor(&self, x: usize, y: usize) -> Floor {
        self.floor[index(x, y)]
    }

    pub fn block(&self, x: usize, y: usize) -> Block {
        self.block[index(x, y)]
    }

    pub fn set_floor(&mut self, x: usize, y: usize, floor: Floor) {
        self.floor[index(x, y)] = floor;
    }

    pub fn set_block(&mut self, x: usize, y: usize, block: Block) {
        self.block[index(x, y)] = block;
    }
}

// Marker on every tile entity (floor and block alike).
#[derive(Component)]
pub struct Tile;

// The grid cell a tile entity belongs to, so the death observer can find it in the TileMap
// without inverting world coordinates.
#[derive(Component)]
pub struct TileCoord {
    pub x: usize,
    pub y: usize,
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileMap>();
        app.add_systems(Startup, generate_map);
        app.add_observer(on_block_death);
    }
}

// Builds the whole map by running the generation stages in order over the TileMap resource, then
// spawns the entities that render the final state.
fn generate_map(mut map: ResMut<TileMap>, mut commands: Commands) {
    let seed: u32 = rand::rng().random();

    let cave_noise = build_noise(seed, 5.0, 0.45);
    let ore_noise = build_noise(seed + 1, 8.0, 0.5);

    stage_base(&mut map, &cave_noise); // Stage 1: floor everywhere + walls
    stage_ore(&mut map, &ore_noise); // Stage 2: upgrade some walls to ore
    stage_carve(&mut map); // Stage 3: clear a space for the player
    spawn_tiles(&map, &mut commands);
}

fn build_noise(seed: u32, frequency: f64, persistence: f64) -> NoiseMap {
    let mut fbm = Fbm::<OpenSimplex>::new(seed);
    fbm.frequency = frequency;
    fbm.persistence = persistence;
    PlaneMapBuilder::<_, 3>::new(&fbm)
        .set_size(MAP_WIDTH, MAP_HEIGHT)
        .build()
}

// Stage 1: lay floor under every cell, and place a wall block wherever the cave noise is solid.
// Open space is simply a floor cell with no block.
fn stage_base(map: &mut TileMap, cave_noise: &NoiseMap) {
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            map.set_floor(x, y, Floor::Floor);
            if cave_noise.get_value(x, y) > 0.0 {
                map.set_block(x, y, Block::Wall);
            }
        }
    }
}

// Stage 2: upgrade existing wall blocks to ore where the ore noise is high. Ore never appears in
// open space because we only touch cells that already hold a wall.
fn stage_ore(map: &mut TileMap, ore_noise: &NoiseMap) {
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            if map.block(x, y) == Block::Wall && ore_noise.get_value(x, y) > 0.2 {
                map.set_block(x, y, Block::Ore);
            }
        }
    }
}

// Stage 3: clear a square of blocks at the map center so the player (spawned at world origin)
// starts in open, walkable space.
fn stage_carve(map: &mut TileMap) {
    let cx = MAP_WIDTH / 2;
    let cy = MAP_HEIGHT / 2;
    for y in cy.saturating_sub(CARVE_RADIUS)..=(cy + CARVE_RADIUS).min(MAP_HEIGHT - 1) {
        for x in cx.saturating_sub(CARVE_RADIUS)..=(cx + CARVE_RADIUS).min(MAP_WIDTH - 1) {
            map.set_block(x, y, Block::None);
            map.set_floor(x, y, Floor::Floor);
        }
    }
}

// Reads the finished map and spawns the sprite entities. A solid cell spawns both a floor entity
// underneath (z = -1) and a block entity on top (z = 0).
fn spawn_tiles(map: &TileMap, commands: &mut Commands) {
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let pos = cell_to_world(x, y);

            if map.floor(x, y) == Floor::Floor {
                spawn_floor_tile(commands, pos, x, y);
            }

            match map.block(x, y) {
                Block::Wall => spawn_wall_tile(commands, pos, x, y),
                Block::Ore => spawn_ore_tile(commands, pos, x, y),
                Block::None => {}
            }
        }
    }
}

// Grid cell -> world position, centered on the origin.
fn cell_to_world(x: usize, y: usize) -> Vec2 {
    let start_x = -(MAP_WIDTH as f32) * TILE_SIZE / 2.0;
    let start_y = -(MAP_HEIGHT as f32) * TILE_SIZE / 2.0;
    Vec2::new(
        start_x + x as f32 * TILE_SIZE,
        start_y + y as f32 * TILE_SIZE,
    )
}

fn spawn_floor_tile(commands: &mut Commands, translation: Vec2, x: usize, y: usize) {
    commands.spawn((
        Tile,
        TileCoord { x, y },
        Transform::from_translation(translation.extend(-1.0)),
        Sprite {
            custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)),
            color: Color::hsl(250.0, 0.2, 0.15), // Dark blue
            ..default()
        },
    ));
}

fn spawn_wall_tile(commands: &mut Commands, translation: Vec2, x: usize, y: usize) {
    commands.spawn((
        Tile,
        TileCoord { x, y },
        Transform::from_translation(translation.extend(0.0)),
        Sprite {
            custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)),
            color: Color::hsl(165.0, 0.1, 0.35), // Light blue
            ..default()
        },
        Collidable {
            shape: Shape::square(TILE_SIZE),
        },
        Health::new(1.0),
    ));
}

fn spawn_ore_tile(commands: &mut Commands, translation: Vec2, x: usize, y: usize) {
    commands.spawn((
        Tile,
        TileCoord { x, y },
        Transform::from_translation(translation.extend(0.0)),
        Sprite {
            custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)),
            color: Color::hsl(45.0, 0.7, 0.25), // Golden
            ..default()
        },
        Collidable {
            shape: Shape::square(TILE_SIZE),
        },
        Health::new(3.0),
    ));
}

// When a breakable block dies, clear its cell in the map and despawn it. The floor entity already
// sits underneath at z = -1, so it is revealed with nothing more to spawn. The With<Tile> guard
// plus the fact that only block entities carry Health means this only ever fires on blocks.
fn on_block_death(
    death: On<Death>,
    blocks: Query<&TileCoord, With<Tile>>,
    mut map: ResMut<TileMap>,
    mut commands: Commands,
) {
    if let Ok(coord) = blocks.get(death.entity) {
        map.set_block(coord.x, coord.y, Block::None);
        commands.entity(death.entity).despawn();
    }
}
