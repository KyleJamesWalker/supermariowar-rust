//! Port of src/server/Clock.h

pub type TimePoint = std::time::Instant;

#[inline]
pub fn time_now() -> TimePoint {
    std::time::Instant::now()
}
