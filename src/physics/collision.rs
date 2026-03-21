use bevy::prelude::*;

use crate::core::constants::*;
use super::components::Collider;

/// Checks whether two AABB colliders overlap given their world positions.
pub fn aabb_overlap(
    pos_a: &Vec3,
    col_a: &Collider,
    pos_b: &Vec3,
    col_b: &Collider,
) -> bool {
    let (a_left, a_top, a_right, a_bottom) = col_a.world_rect(pos_a);
    let (b_left, b_top, b_right, b_bottom) = col_b.world_rect(pos_b);

    a_left < b_right && a_right > b_left && a_top < b_bottom && a_bottom > b_top
}

/// Checks for AABB overlap accounting for horizontal screen wrapping.
/// Returns true if the two colliders overlap at either the direct position or
/// a wrapped position (shifted by SCREEN_WIDTH + some margin).
pub fn aabb_overlap_wrapped(
    pos_a: &Vec3,
    col_a: &Collider,
    pos_b: &Vec3,
    col_b: &Collider,
) -> bool {
    // Direct check
    if aabb_overlap(pos_a, col_a, pos_b, col_b) {
        return true;
    }

    let wrap_amount = SCREEN_WIDTH;

    // Check A wrapped right
    let mut wrapped = *pos_a;
    wrapped.x += wrap_amount;
    if aabb_overlap(&wrapped, col_a, pos_b, col_b) {
        return true;
    }

    // Check A wrapped left
    wrapped.x = pos_a.x - wrap_amount;
    if aabb_overlap(&wrapped, col_a, pos_b, col_b) {
        return true;
    }

    false
}

/// Returns the overlap penetration vector to push entity A out of entity B.
/// Returns None if there is no overlap.
pub fn aabb_penetration(
    pos_a: &Vec3,
    col_a: &Collider,
    pos_b: &Vec3,
    col_b: &Collider,
) -> Option<Vec2> {
    let (a_left, a_top, a_right, a_bottom) = col_a.world_rect(pos_a);
    let (b_left, b_top, b_right, b_bottom) = col_b.world_rect(pos_b);

    let overlap_x = a_right.min(b_right) - a_left.max(b_left);
    let overlap_y = a_bottom.min(b_bottom) - a_top.max(b_top);

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

/// Returns the overlap penetration vector accounting for horizontal screen wrapping.
/// Tries direct position and wrapped positions, returning the penetration for
/// whichever overlap is found first (or None if no overlap).
pub fn aabb_penetration_wrapped(
    pos_a: &Vec3,
    col_a: &Collider,
    pos_b: &Vec3,
    col_b: &Collider,
) -> Option<Vec2> {
    // Direct check
    if let Some(pen) = aabb_penetration(pos_a, col_a, pos_b, col_b) {
        return Some(pen);
    }

    let wrap_amount = SCREEN_WIDTH;

    // Check A wrapped right
    let mut wrapped = *pos_a;
    wrapped.x += wrap_amount;
    if let Some(pen) = aabb_penetration(&wrapped, col_a, pos_b, col_b) {
        return Some(pen);
    }

    // Check A wrapped left
    wrapped.x = pos_a.x - wrap_amount;
    if let Some(pen) = aabb_penetration(&wrapped, col_a, pos_b, col_b) {
        return Some(pen);
    }

    None
}

/// Computes the shortest horizontal distance between two x-positions, accounting
/// for screen wrapping. Returns a signed value (positive = b is to the right of a).
pub fn wrapped_distance_x(x_a: f32, x_b: f32) -> f32 {
    let wrap_width = SCREEN_WIDTH;
    let direct = x_b - x_a;

    // Check the three candidate distances: direct, and wrapped in both directions
    let candidates = [direct, direct + wrap_width, direct - wrap_width];
    let mut best = direct;
    let mut best_abs = direct.abs();

    for &c in &candidates[1..] {
        if c.abs() < best_abs {
            best = c;
            best_abs = c.abs();
        }
    }

    best
}

/// Performs a sweep test for a moving AABB against a static AABB.
/// Returns the time of first contact (0.0..=1.0) and the contact normal, or None if no hit.
///
/// `displacement` is the movement vector of entity A during this frame.
pub fn sweep_aabb(
    pos_a: &Vec3,
    col_a: &Collider,
    displacement: &Vec2,
    pos_b: &Vec3,
    col_b: &Collider,
) -> Option<(f32, Vec2)> {
    // Early out for zero displacement
    if displacement.x == 0.0 && displacement.y == 0.0 {
        if aabb_overlap(pos_a, col_a, pos_b, col_b) {
            return Some((0.0, Vec2::ZERO));
        }
        return None;
    }

    let (a_left, a_top, a_right, a_bottom) = col_a.world_rect(pos_a);
    let (b_left, b_top, b_right, b_bottom) = col_b.world_rect(pos_b);

    // Compute the Minkowski sum expanded box
    let expanded_left = b_left - (a_right - a_left);
    let expanded_top = b_top - (a_bottom - a_top);
    let expanded_right = b_right;
    let expanded_bottom = b_bottom;

    // Ray cast from A's top-left corner against the expanded box
    let origin = Vec2::new(a_left, a_top);

    let mut t_enter = f32::NEG_INFINITY;
    let mut t_exit = f32::INFINITY;
    let mut enter_normal = Vec2::ZERO;

    // X axis
    if displacement.x != 0.0 {
        let inv_dx = 1.0 / displacement.x;
        let mut t1 = (expanded_left - origin.x) * inv_dx;
        let mut t2 = (expanded_right - origin.x) * inv_dx;
        let mut normal = Vec2::new(-1.0, 0.0);

        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
            normal.x = 1.0;
        }

        if t1 > t_enter {
            t_enter = t1;
            enter_normal = normal;
        }
        t_exit = t_exit.min(t2);
    } else {
        // Not moving on X: check static overlap
        if origin.x < expanded_left || origin.x > expanded_right {
            return None;
        }
    }

    // Y axis
    if displacement.y != 0.0 {
        let inv_dy = 1.0 / displacement.y;
        let mut t1 = (expanded_top - origin.y) * inv_dy;
        let mut t2 = (expanded_bottom - origin.y) * inv_dy;
        let mut normal = Vec2::new(0.0, -1.0);

        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
            normal.y = 1.0;
        }

        if t1 > t_enter {
            t_enter = t1;
            enter_normal = normal;
        }
        t_exit = t_exit.min(t2);
    } else {
        if origin.y < expanded_top || origin.y > expanded_bottom {
            return None;
        }
    }

    // No intersection if enter > exit or exit < 0 or enter > 1
    if t_enter > t_exit || t_exit < 0.0 || t_enter > 1.0 {
        return None;
    }

    let t = t_enter.max(0.0);
    Some((t, enter_normal))
}
