use crate::modules::player::{PLAYER_SIZE, Player};
use crate::modules::terrain::TILE_SIZE;
use bevy::math::bounding::{Aabb2d, BoundingVolume, IntersectsVolume};
use bevy::prelude::*;

#[derive(Component)]
pub struct Collidable;

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, collision_system);
    }
}

fn collision_system(
    mut player_transform: Single<&mut Transform, (With<Player>, Without<Collidable>)>,
    mut collidable_query: Query<&Transform, (With<Collidable>, Without<Player>)>,
) {
    for collidable_transform in &mut collidable_query {
        let player_bb = Aabb2d::new(
            player_transform.translation.truncate(),
            PLAYER_SIZE * 0.5 * player_transform.scale.truncate().abs(),
        );

        let collidable_bb = Aabb2d::new(
            collidable_transform.translation.truncate(),
            TILE_SIZE * 0.5 * collidable_transform.scale.truncate().abs(),
        );

        if !collidable_bb.intersects(&player_bb) {
            continue;
        };

        let delta = player_bb.center() - collidable_bb.center();
        let combined = player_bb.half_size() + collidable_bb.half_size();
        let overlap = combined - delta.abs(); // both components > 0 when intersecting

        if overlap.x < overlap.y {
            player_transform.translation.x += overlap.x * delta.x.signum();
        } else {
            player_transform.translation.y += overlap.y * delta.y.signum();
        }
    }
}
