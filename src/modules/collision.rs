use crate::modules::map::Tile;
use crate::modules::player::Player;
use bevy::math::bounding::{Aabb2d, BoundingCircle, BoundingVolume, IntersectsVolume};
use bevy::prelude::*;

#[derive(Clone, Copy)]
pub enum Shape {
    Rect { half_size: Vec2 },
    Circle { radius: f32 },
}

impl Shape {
    pub fn square(size: f32) -> Self {
        Shape::Rect {
            half_size: Vec2::splat(size * 0.5),
        }
    }

    pub fn circle(radius: f32) -> Self {
        Shape::Circle { radius }
    }

    // Enclosing axis-aligned box, used for physical penetration resolution.
    fn aabb(&self, center: Vec2, scale: Vec2) -> Aabb2d {
        let half_size = match self {
            Shape::Rect { half_size } => *half_size,
            Shape::Circle { radius } => Vec2::splat(*radius),
        };
        Aabb2d::new(center, half_size * scale.abs())
    }

    // Bounding circle for circle shapes (uses the largest scaled axis as radius).
    fn circle_vol(&self, center: Vec2, scale: Vec2) -> BoundingCircle {
        let radius = match self {
            Shape::Rect { half_size } => half_size.length(),
            Shape::Circle { radius } => *radius,
        };
        BoundingCircle::new(center, radius * scale.abs().max_element())
    }
}

#[derive(Component)]
pub struct Collidable {
    pub shape: Shape,
}

// Boolean shape-vs-shape intersection across all rect/circle combinations.
fn shapes_intersect(
    a_pos: Vec2,
    a: &Collidable,
    a_scale: Vec2,
    b_pos: Vec2,
    b: &Collidable,
    b_scale: Vec2,
) -> bool {
    match (a.shape, b.shape) {
        (Shape::Rect { .. }, Shape::Rect { .. }) => a
            .shape
            .aabb(a_pos, a_scale)
            .intersects(&b.shape.aabb(b_pos, b_scale)),
        (Shape::Rect { .. }, Shape::Circle { .. }) => a
            .shape
            .aabb(a_pos, a_scale)
            .intersects(&b.shape.circle_vol(b_pos, b_scale)),
        (Shape::Circle { .. }, Shape::Rect { .. }) => a
            .shape
            .circle_vol(a_pos, a_scale)
            .intersects(&b.shape.aabb(b_pos, b_scale)),
        (Shape::Circle { .. }, Shape::Circle { .. }) => a
            .shape
            .circle_vol(a_pos, a_scale)
            .intersects(&b.shape.circle_vol(b_pos, b_scale)),
    }
}

#[derive(Component)]
pub struct Health {
    pub current: f32,
    // Kept for upcoming consumers (health bars, healing/regen); not read yet.
    #[allow(dead_code)]
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }
}

#[derive(Component)]
pub struct ContactDamage {
    pub amount: f32,
}

// Marker: a damager that is consumed after the damage pass (a one-shot pulse, e.g. the
// player smash). Persistent damagers such as enemies simply omit it.
#[derive(Component)]
pub struct DespawnOnHit;

// Decoupled death signal. The field named `entity` is the EntityEvent target, so reaction
// systems in other modules (e.g. the map turning a dead block into floor) observe it without
// this module knowing anything about them.
#[derive(EntityEvent)]
pub struct Death {
    pub entity: Entity,
}

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (collision_system, damage_system));
    }
}

fn collision_system(
    player_query: Single<(&mut Transform, &Collidable), (With<Player>, Without<Tile>)>,
    mut tile_query: Query<(&Transform, &Collidable), (With<Tile>, Without<Player>)>,
) {
    let (mut player_transform, player_collidable) = player_query.into_inner();

    for (tile_transform, tile_collidable) in &mut tile_query {
        let player_bb = player_collidable.shape.aabb(
            player_transform.translation.truncate(),
            player_transform.scale.truncate(),
        );

        let tile_bb = tile_collidable.shape.aabb(
            tile_transform.translation.truncate(),
            tile_transform.scale.truncate(),
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

// Applies contact damage from every damager to every overlapping entity with Health, and
// signals a Death when health is depleted. Reaction to that death (despawn, loot, floor tile,
// ...) is owned by other modules via Death observers, so this stays generic.
fn damage_system(
    damagers: Query<(Entity, &Transform, &Collidable, &ContactDamage, Has<DespawnOnHit>)>,
    mut targets: Query<(Entity, &Transform, &Collidable, &mut Health)>,
    mut commands: Commands,
) {
    for (damager_entity, damager_transform, damager_collidable, damage, despawn_on_hit) in &damagers
    {
        for (target_entity, target_transform, target_collidable, mut health) in &mut targets {
            // An entity can be both a damager and a target (e.g. an enemy), so never let one
            // damage itself.
            if damager_entity == target_entity {
                continue;
            }
            // Skip the already-dead: this both avoids re-triggering Death the same frame (the
            // despawn from the observer is deferred to the next sync point) and ignores entities
            // awaiting their reaction.
            if health.current <= 0.0 {
                continue;
            }

            let intersects = shapes_intersect(
                damager_transform.translation.truncate(),
                damager_collidable,
                damager_transform.scale.truncate(),
                target_transform.translation.truncate(),
                target_collidable,
                target_transform.scale.truncate(),
            );
            if !intersects {
                continue;
            }

            health.current -= damage.amount;
            if health.current <= 0.0 {
                commands.trigger(Death {
                    entity: target_entity,
                });
            }
        }

        if despawn_on_hit {
            commands.entity(damager_entity).despawn();
        }
    }
}
