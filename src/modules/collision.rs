use crate::modules::player::Player;
use crate::modules::terrain::{Tile, spawn_floor_tile};
use bevy::math::bounding::{Aabb2d, BoundingVolume, IntersectsVolume};
use bevy::prelude::*;

#[derive(Component)]
pub struct Collidable {
    pub size: f32,
}

#[derive(Component)]
pub struct Breakable {
    pub health: f32,
}

#[derive(Component)]
pub struct Breaker {
    pub damage: f32,
}

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (collision_system, breaking_system));
    }
}

fn collision_system(
    player_query: Single<(&mut Transform, &Collidable), (With<Player>, Without<Tile>)>,
    mut tile_query: Query<(&Transform, &Collidable), (With<Tile>, Without<Player>)>,
) {
    let (mut player_transform, player_collidable) = player_query.into_inner();

    for (tile_transform, tile_collidable) in &mut tile_query {
        let player_bb = Aabb2d::new(
            player_transform.translation.truncate(),
            player_collidable.size * 0.5 * player_transform.scale.truncate().abs(),
        );

        let tile_bb = Aabb2d::new(
            tile_transform.translation.truncate(),
            tile_collidable.size * 0.5 * tile_transform.scale.truncate().abs(),
        );

        if !tile_bb.intersects(&player_bb) {
            continue;
        };

        let delta = player_bb.center() - tile_bb.center();
        let combined = player_bb.half_size() + tile_bb.half_size();
        let overlap = combined - delta.abs(); // both components > 0 when intersecting

        if overlap.x < overlap.y {
            player_transform.translation.x += overlap.x * delta.x.signum();
        } else {
            player_transform.translation.y += overlap.y * delta.y.signum();
        }
    }
}

fn breaking_system(
    mut breakable_query: Query<(Entity, &Transform, &mut Breakable), Without<Breaker>>,
    breaker_query: Query<(Entity, &Transform, &Breaker), Without<Breakable>>,
    mut commands: Commands,
) {
    for (breaker_entity, breaker_transform, breaker) in breaker_query {
        for (breakable_entity, breakable_transform, mut breakable) in &mut breakable_query {
            let distance = breaker_transform
                .translation
                .truncate()
                .distance(breakable_transform.translation.truncate());
            if distance < 48.0 {
                breakable.health -= breaker.damage;
                if breakable.health <= 0.0 {
                    spawn_floor_tile(&mut commands, breakable_transform.translation.truncate());
                    commands.entity(breakable_entity).despawn();
                }
            }
        }
        commands.entity(breaker_entity).despawn();
    }
}
