use bevy::prelude::*;
use noise::{
    Fbm, Perlin,
    utils::{NoiseMapBuilder, PlaneMapBuilder},
};

#[derive(Resource)]
struct GameWorld {
    noise_generator: Fbm<Perlin>,
}

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GameWorld {
            noise_generator: Fbm::<Perlin>::new(0),
        });
        app.add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands, world: Res<GameWorld>) {
    // Draw procedural map
    let noise_map = PlaneMapBuilder::<_, 3>::new(&world.noise_generator).build();
    let (grid_width, grid_height) = noise_map.size();
    let tile_size = 32_f32;
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
                    color: Color::hsl((360.0 * (val + 1.0) / 2.0) as f32, 0.5, 0.5),
                    ..default()
                },
                Transform::from_translation(Vec3::new(x, y, -1.0)),
            ));
        }
    }
}
