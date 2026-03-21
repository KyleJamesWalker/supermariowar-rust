use bevy::prelude::*;

use super::components::Collider;

/// Checks whether two AABB colliders overlap given their world positions.
pub fn aabb_overlap(
    pos_a: &Vec3,
    col_a: &Collider,
    pos_b: &Vec3,
    col_b: &Collider,
) -> bool {
    let a_left = pos_a.x + col_a.offset_x;
    let a_right = a_left + col_a.width;
    let a_top = pos_a.y + col_a.offset_y;
    let a_bottom = a_top + col_a.height;

    let b_left = pos_b.x + col_b.offset_x;
    let b_right = b_left + col_b.width;
    let b_top = pos_b.y + col_b.offset_y;
    let b_bottom = b_top + col_b.height;

    a_left < b_right && a_right > b_left && a_top < b_bottom && a_bottom > b_top
}

/// Returns the overlap penetration vector to push entity A out of entity B.
/// Returns None if there is no overlap.
pub fn aabb_penetration(
    pos_a: &Vec3,
    col_a: &Collider,
    pos_b: &Vec3,
    col_b: &Collider,
) -> Option<Vec2> {
    let a_left = pos_a.x + col_a.offset_x;
    let a_right = a_left + col_a.width;
    let a_top = pos_a.y + col_a.offset_y;
    let a_bottom = a_top + col_a.height;

    let b_left = pos_b.x + col_b.offset_x;
    let b_right = b_left + col_b.width;
    let b_top = pos_b.y + col_b.offset_y;
    let b_bottom = b_top + col_b.height;

    let overlap_x = (a_right.min(b_right)) - (a_left.max(b_left));
    let overlap_y = (a_bottom.min(b_bottom)) - (a_top.max(b_top));

    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return None;
    }

    // Resolve along the axis of least penetration
    let center_a = Vec2::new(a_left + col_a.width / 2.0, a_top + col_a.height / 2.0);
    let center_b = Vec2::new(b_left + col_b.width / 2.0, b_top + col_b.height / 2.0);
    let diff = center_a - center_b;

    if overlap_x < overlap_y {
        let sign = if diff.x > 0.0 { 1.0 } else { -1.0 };
        Some(Vec2::new(overlap_x * sign, 0.0))
    } else {
        let sign = if diff.y > 0.0 { 1.0 } else { -1.0 };
        Some(Vec2::new(0.0, overlap_y * sign))
    }
}
