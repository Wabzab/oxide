mod modules;

use bevy::prelude::*;
use modules::{PlayerPlugin, TerrainPlugin};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PlayerPlugin, TerrainPlugin))
        .run();
}
