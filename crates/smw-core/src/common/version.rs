//! Port of src/common/Version.h

#[derive(Clone, Copy, Debug, Default)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
    pub build: u8,
}

impl Version {
    pub const fn as_u32(&self) -> u32 {
        ((self.major as u32) << 24) | ((self.minor as u32) << 16) | ((self.patch as u32) << 8) | self.build as u32
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.as_u32() == other.as_u32()
    }
}
impl Eq for Version {}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.as_u32().cmp(&other.as_u32()))
    }
}

pub const GAME_VERSION: Version = Version { major: 2, minor: 0, patch: 0, build: 1 };
