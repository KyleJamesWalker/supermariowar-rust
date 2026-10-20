//! The lobby server's monotonic clock.

pub type TimePoint = std::time::Instant;

#[inline]
pub fn time_now() -> TimePoint {
    std::time::Instant::now()
}
