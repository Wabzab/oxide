mod modules;

use bevy::prelude::*;
use modules::{CollisionPlugin, PlayerPlugin, TerrainPlugin};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CollisionPlugin, PlayerPlugin, TerrainPlugin))
        .run();
}
