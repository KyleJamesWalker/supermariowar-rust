//! Port of src/common/util/ContainerHelpers.h

pub fn contains<T: PartialEq>(list: &[T], item: &T) -> bool {
    list.iter().any(|x| x == item)
}

// C++ uses std::sort (unstable, libc++ introsort); equal elements may land in a different order.
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
