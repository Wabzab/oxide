mod modules;

use bevy::prelude::*;
use modules::{CollisionPlugin, MapPlugin, PlayerPlugin};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CollisionPlugin, PlayerPlugin, MapPlugin))
        .run();
}
