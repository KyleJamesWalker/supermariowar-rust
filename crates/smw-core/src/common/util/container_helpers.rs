//! Slice helpers for the `contains` and strict-weak-ordering `sort` calls the game makes.

pub fn contains<T: PartialEq>(list: &[T], item: &T) -> bool {
    list.iter().any(|x| x == item)
}

/// An unstable sort by a less-than predicate; equal elements may land in any order.
pub fn sort<T, F: FnMut(&T, &T) -> bool>(list: &mut [T], mut compare: F) {
    list.sort_unstable_by(|a, b| {
        if compare(a, b) {
            std::cmp::Ordering::Less
        } else if compare(b, a) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
}
