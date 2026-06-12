use crate::modules::collision::{Collidable, Death, Health, Shape};
use crate::modules::player::Player;
use bevy::prelude::*;
use rand::distr::{Distribution, weighted::WeightedIndex};
use rand::prelude::*;
use std::collections::{HashMap, HashSet};

pub const TILE_SIZE: f32 = 32.0;
pub const CHUNK_SIZE: i32 = 8; // cells per chunk edge
const CHUNK_CELLS: usize = (CHUNK_SIZE * CHUNK_SIZE) as usize; // 64
const DEFAULT_RENDER_DISTANCE: i32 = 3; // N: loaded area is N x N chunks, centered on the player. Keep odd.
const CARVE_RADIUS: i32 = 4; // cells, half-extent of the square clearing around the spawn origin

// The two layers store one cell value each. Floor is the walkable ground that exists everywhere;
// Block is what sits on top of it (a wall, ore, or nothing).
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
    Rock,
    Iron,
    Gold,
}

// Position of a chunk in the infinite chunk grid (can be negative).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ChunkCoord {
    x: i32,
    y: i32,
}

// Global cell coordinate, one per tile (can be negative).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct CellCoord {
    x: i32,
    y: i32,
}

impl CellCoord {
    // Which chunk this cell belongs to. div_euclid (not /) so negative coords map correctly.
    fn chunk(self) -> ChunkCoord {
        ChunkCoord {
            x: self.x.div_euclid(CHUNK_SIZE),
            y: self.y.div_euclid(CHUNK_SIZE),
        }
    }

    // Flat index of this cell within its chunk. rem_euclid (not %) so negatives land in 0..CHUNK_SIZE.
    fn index_in_chunk(self) -> usize {
        let lx = self.x.rem_euclid(CHUNK_SIZE);
        let ly = self.y.rem_euclid(CHUNK_SIZE);
        (ly * CHUNK_SIZE + lx) as usize
    }
}

impl ChunkCoord {
    // Global cell coordinate for a local flat index within this chunk.
    fn cell_at(self, idx: usize) -> CellCoord {
        let lx = (idx as i32) % CHUNK_SIZE;
        let ly = (idx as i32) / CHUNK_SIZE;
        CellCoord {
            x: self.x * CHUNK_SIZE + lx,
            y: self.y * CHUNK_SIZE + ly,
        }
    }
}

// Transient base terrain for a single chunk. Regenerated deterministically each time the chunk
// streams in; never stored, since the spawned entities plus the edit overlay are the live state.
struct Chunk {
    floor: [Floor; CHUNK_CELLS],
    block: [Block; CHUNK_CELLS],
}

// The streaming world. Named ChunkMap (not World) to avoid colliding with bevy::prelude::World.
#[derive(Resource, Default)]
pub struct ChunkMap {
    // Loaded chunks -> their spawned entities, so a chunk can be despawned as a unit.
    loaded: HashMap<ChunkCoord, Vec<Entity>>,
    // Persistent player edits: chunk -> (in-chunk index -> overriding block). Survives stream-out/in.
    edits: HashMap<ChunkCoord, HashMap<usize, Block>>,
    // The chunk the player occupied last frame, to detect crossings.
    current_chunk: Option<ChunkCoord>,
}

// One ore type's spawn rules.
pub struct OreConfig {
    pub block: Block,
    pub chunk_probability: f64, // chance this ore seeds a vein in a given chunk
    pub vein_sizes: Vec<(usize, f64)>, // weighted (vein cell count, weight)
}

// Generation config. The seed makes the whole world reproducible; ores are placed in list order,
// so earlier (rarer) ores win cells over later ones.
#[derive(Resource)]
pub struct MapConfig {
    pub seed: u64,
    pub render_distance: i32, // N, diameter in chunks; keep odd so the player's chunk is centered
    pub ores: Vec<OreConfig>,
}

impl Default for MapConfig {
    fn default() -> Self {
        let seed: u64 = rand::rng().random();
        info!("world seed: {seed}");
        Self {
            seed,
            render_distance: DEFAULT_RENDER_DISTANCE,
            ores: vec![
                OreConfig {
                    block: Block::Gold,
                    chunk_probability: 0.25,
                    vein_sizes: vec![(3, 4.0), (4, 2.0), (5, 1.0)],
                },
                OreConfig {
                    block: Block::Iron,
                    chunk_probability: 0.6,
                    vein_sizes: vec![(4, 3.0), (6, 2.0), (8, 1.0)],
                },
            ],
        }
    }
}

// Marker on every tile entity (floor and block alike).
#[derive(Component)]
pub struct Tile;

// The global grid cell a tile entity belongs to, so the death observer can record the edit in the
// right chunk overlay without inverting world coordinates.
#[derive(Component)]
pub struct TileCoord {
    cell: CellCoord,
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkMap>();
        app.init_resource::<MapConfig>(); // Default logs the seed
        app.add_systems(Startup, carve_origin);
        app.add_systems(Update, stream_chunks);
        app.add_observer(on_block_death);
    }
}

// Grid cell -> world position. Cell (0, 0) anchors the world origin, where the player spawns.
fn cell_to_world(cell: CellCoord) -> Vec2 {
    Vec2::new(cell.x as f32 * TILE_SIZE, cell.y as f32 * TILE_SIZE)
}

// World position -> nearest grid cell. Exact inverse of cell_to_world on cell centers.
fn world_to_cell(pos: Vec2) -> CellCoord {
    CellCoord {
        x: (pos.x / TILE_SIZE).round() as i32,
        y: (pos.y / TILE_SIZE).round() as i32,
    }
}

// Deterministic per-chunk seed: mixes the world seed with the (signed) chunk coords via a
// splitmix64-style hash so each chunk is independent and reproducible, with no axis correlation.
fn chunk_seed(world_seed: u64, c: ChunkCoord) -> u64 {
    let mut h = world_seed;
    h = h
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(c.x as u32 as u64);
    h = h
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(c.y as u32 as u64);
    h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 31)
}

fn chunk_rng(world_seed: u64, c: ChunkCoord) -> StdRng {
    StdRng::seed_from_u64(chunk_seed(world_seed, c))
}

// Base terrain: floor everywhere, solid rock on top. Ore and the edit overlay are layered on after.
fn generate_base(chunk: &mut Chunk) {
    chunk.floor = [Floor::Floor; CHUNK_CELLS];
    chunk.block = [Block::Rock; CHUNK_CELLS];
}

// Seed ore veins into the chunk. For each ore (in priority order), maybe place one vein: pick a
// weighted size, a random origin cell, then random-walk that many cells, converting only Rock and
// staying inside the chunk. Converting only Rock means earlier ores in the list keep their cells.
fn generate_ores(chunk: &mut Chunk, rng: &mut StdRng, ores: &[OreConfig]) {
    for ore in ores {
        if !rng.random_bool(ore.chunk_probability) {
            continue;
        }

        let weights = ore.vein_sizes.iter().map(|(_, w)| *w);
        let dist = WeightedIndex::new(weights).unwrap();
        let size = ore.vein_sizes[dist.sample(rng)].0;

        let mut lx = rng.random_range(0..CHUNK_SIZE);
        let mut ly = rng.random_range(0..CHUNK_SIZE);

        let mut placed = 0;
        let mut attempts = 0;
        let max_attempts = size * 16;
        while placed < size && attempts < max_attempts {
            attempts += 1;
            let idx = (ly * CHUNK_SIZE + lx) as usize;
            if chunk.block[idx] == Block::Rock {
                chunk.block[idx] = ore.block;
                placed += 1;
            }
            // Step in a random cardinal direction, clamped inside the chunk.
            match rng.random_range(0..4) {
                0 => lx = (lx + 1).min(CHUNK_SIZE - 1),
                1 => lx = (lx - 1).max(0),
                2 => ly = (ly + 1).min(CHUNK_SIZE - 1),
                _ => ly = (ly - 1).max(0),
            }
        }
    }
}

// Apply persistent player edits on top of the generated base.
fn apply_overlay(chunk: &mut Chunk, edits: Option<&HashMap<usize, Block>>) {
    if let Some(edits) = edits {
        for (&idx, &block) in edits {
            chunk.block[idx] = block;
        }
    }
}

// Clear a square of blocks around the spawn origin so the player isn't buried. Written into the
// edit overlay so it persists across stream-out/in and is reapplied on every reload automatically.
fn carve_origin(mut map: ResMut<ChunkMap>) {
    for dy in -CARVE_RADIUS..=CARVE_RADIUS {
        for dx in -CARVE_RADIUS..=CARVE_RADIUS {
            let cell = CellCoord { x: dx, y: dy };
            map.edits
                .entry(cell.chunk())
                .or_default()
                .insert(cell.index_in_chunk(), Block::None);
        }
    }
}

// Generate a chunk (base -> ore -> overlay) and spawn its tile entities. Returns the entities so the
// streamer can despawn the whole chunk later.
fn load_chunk(
    coord: ChunkCoord,
    config: &MapConfig,
    edits: &HashMap<ChunkCoord, HashMap<usize, Block>>,
    commands: &mut Commands,
) -> Vec<Entity> {
    let mut chunk = Chunk {
        floor: [Floor::None; CHUNK_CELLS],
        block: [Block::None; CHUNK_CELLS],
    };
    let mut rng = chunk_rng(config.seed, coord);

    generate_base(&mut chunk);
    generate_ores(&mut chunk, &mut rng, &config.ores);
    apply_overlay(&mut chunk, edits.get(&coord));

    let mut entities = Vec::new();
    for idx in 0..CHUNK_CELLS {
        let cell = coord.cell_at(idx);
        let pos = cell_to_world(cell);

        if chunk.floor[idx] == Floor::Floor {
            entities.push(spawn_floor_tile(commands, pos, cell));
        }

        match chunk.block[idx] {
            Block::Rock => entities.push(spawn_rock_tile(commands, pos, cell)),
            Block::Iron => entities.push(spawn_iron_tile(commands, pos, cell)),
            Block::Gold => entities.push(spawn_gold_tile(commands, pos, cell)),
            Block::None => {}
        }
    }
    entities
}

// Keep the N x N chunks around the player loaded: spawn newly-needed chunks and despawn ones that
// have fallen out of range. Only re-diffs when the player crosses into a new chunk.
fn stream_chunks(
    player: Single<&Transform, With<Player>>,
    config: Res<MapConfig>,
    mut map: ResMut<ChunkMap>,
    mut commands: Commands,
) {
    let player_chunk = world_to_cell(player.translation.truncate()).chunk();
    if map.current_chunk == Some(player_chunk) {
        return;
    }
    map.current_chunk = Some(player_chunk);

    let half = config.render_distance / 2; // odd N -> exactly N chunks per axis
    let mut desired = HashSet::new();
    for dy in -half..=half {
        for dx in -half..=half {
            desired.insert(ChunkCoord {
                x: player_chunk.x + dx,
                y: player_chunk.y + dy,
            });
        }
    }

    // Despawn out-of-range chunks. Keep their edits; only base + entities are dropped.
    let to_unload: Vec<ChunkCoord> = map
        .loaded
        .keys()
        .filter(|c| !desired.contains(c))
        .copied()
        .collect();
    for coord in to_unload {
        if let Some(entities) = map.loaded.remove(&coord) {
            for entity in entities {
                commands.entity(entity).despawn();
            }
        }
    }

    // Spawn newly-needed chunks.
    let to_load: Vec<ChunkCoord> = desired
        .iter()
        .filter(|c| !map.loaded.contains_key(c))
        .copied()
        .collect();
    for coord in to_load {
        let entities = load_chunk(coord, &config, &map.edits, &mut commands);
        map.loaded.insert(coord, entities);
    }
}

fn spawn_floor_tile(commands: &mut Commands, translation: Vec2, cell: CellCoord) -> Entity {
    commands
        .spawn((
            Tile,
            TileCoord { cell },
            Transform::from_translation(translation.extend(-1.0)),
            Sprite {
                custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)),
                color: Color::hsl(250.0, 0.2, 0.15), // Dark blue
                ..default()
            },
        ))
        .id()
}

fn spawn_rock_tile(commands: &mut Commands, translation: Vec2, cell: CellCoord) -> Entity {
    commands
        .spawn((
            Tile,
            TileCoord { cell },
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
        ))
        .id()
}

fn spawn_iron_tile(commands: &mut Commands, translation: Vec2, cell: CellCoord) -> Entity {
    commands
        .spawn((
            Tile,
            TileCoord { cell },
            Transform::from_translation(translation.extend(0.0)),
            Sprite {
                custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)),
                color: Color::hsl(5.0, 0.7, 0.25), // Rust
                ..default()
            },
            Collidable {
                shape: Shape::square(TILE_SIZE),
            },
            Health::new(3.0),
        ))
        .id()
}

fn spawn_gold_tile(commands: &mut Commands, translation: Vec2, cell: CellCoord) -> Entity {
    commands
        .spawn((
            Tile,
            TileCoord { cell },
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
        ))
        .id()
}

// When a breakable block dies, record the cell as cleared in its chunk's overlay (so it stays
// cleared across stream-out/in), drop it from the loaded set so it can't be double-despawned, and
// despawn it. The floor entity underneath at z = -1 is revealed with nothing more to spawn.
fn on_block_death(
    death: On<Death>,
    blocks: Query<&TileCoord, With<Tile>>,
    mut map: ResMut<ChunkMap>,
    mut commands: Commands,
) {
    if let Ok(coord) = blocks.get(death.entity) {
        let cell = coord.cell;
        let chunk = cell.chunk();
        map.edits
            .entry(chunk)
            .or_default()
            .insert(cell.index_in_chunk(), Block::None);
        if let Some(entities) = map.loaded.get_mut(&chunk) {
            entities.retain(|&e| e != death.entity);
        }
        commands.entity(death.entity).despawn();
    }
}
