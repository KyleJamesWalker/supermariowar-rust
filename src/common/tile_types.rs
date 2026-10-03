//! Port of src/common/TileTypes.cpp

/// May hold any `u8`: map and tileset files cast arbitrary integers into it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Hash)]
pub struct TileType(pub u8);

#[allow(non_upper_case_globals)]
impl TileType {
    pub const NonSolid: TileType = TileType(0);
    pub const Solid: TileType = TileType(1);
    pub const SolidOnTop: TileType = TileType(2);
    pub const Ice: TileType = TileType(3);
    pub const Death: TileType = TileType(4);
    pub const DeathOnTop: TileType = TileType(5);
    pub const DeathOnBottom: TileType = TileType(6);
    pub const DeathOnLeft: TileType = TileType(7);
    pub const DeathOnRight: TileType = TileType(8);
    pub const IceOnTop: TileType = TileType(9);
    pub const IceDeathOnBottom: TileType = TileType(10);
    pub const IceDeathOnLleft: TileType = TileType(11);
    pub const IceDeathOnRight: TileType = TileType(12);
    pub const SuperDeath: TileType = TileType(13);
    pub const SuperDeathTop: TileType = TileType(14);
    pub const SuperDeathBottom: TileType = TileType(15);
    pub const SuperDeathLeft: TileType = TileType(16);
    pub const SuperDeathRight: TileType = TileType(17);
    pub const PlayerDeath: TileType = TileType(18);
    pub const Gap: TileType = TileType(19);

    /// `static_cast<TileType>(int)`
    pub const fn from_i32(v: i32) -> TileType {
        TileType(v as u8)
    }
}

pub type TileTypeFlag = i32;
pub const tile_flag_nonsolid: TileTypeFlag = 0;
pub const tile_flag_solid: TileTypeFlag = 1;
pub const tile_flag_solid_on_top: TileTypeFlag = 2;
pub const tile_flag_ice: TileTypeFlag = 4;
pub const tile_flag_death_on_top: TileTypeFlag = 8;
pub const tile_flag_death_on_bottom: TileTypeFlag = 16;
pub const tile_flag_death_on_left: TileTypeFlag = 32;
pub const tile_flag_death_on_right: TileTypeFlag = 64;
pub const tile_flag_gap: TileTypeFlag = 128;
pub const tile_flag_super_death_top: TileTypeFlag = 256;
pub const tile_flag_super_death_bottom: TileTypeFlag = 512;
pub const tile_flag_super_death_left: TileTypeFlag = 1024;
pub const tile_flag_super_death_right: TileTypeFlag = 2048;
pub const tile_flag_player_death: TileTypeFlag = 4096;

pub const tile_flag_has_regular_death: TileTypeFlag =
    tile_flag_death_on_top | tile_flag_death_on_bottom | tile_flag_death_on_left | tile_flag_death_on_right;
pub const tile_flag_has_super_death: TileTypeFlag =
    tile_flag_super_death_top | tile_flag_super_death_bottom | tile_flag_super_death_left | tile_flag_super_death_right;
pub const tile_flag_has_death: TileTypeFlag = tile_flag_player_death | tile_flag_has_regular_death | tile_flag_has_super_death;

pub const tile_flag_super_or_player_death_top: TileTypeFlag = tile_flag_player_death | tile_flag_super_death_top;
pub const tile_flag_super_or_player_death_bottom: TileTypeFlag = tile_flag_player_death | tile_flag_super_death_bottom;
pub const tile_flag_super_or_player_death_left: TileTypeFlag = tile_flag_player_death | tile_flag_super_death_left;
pub const tile_flag_super_or_player_death_right: TileTypeFlag = tile_flag_player_death | tile_flag_super_death_right;
pub const tile_flag_player_or_death_on_bottom: TileTypeFlag = tile_flag_player_death | tile_flag_death_on_bottom;

/// Returns `unsigned short` like C++; callers widen it to `int`.
pub const fn tile_to_flags(tiletype: TileType) -> u16 {
    (match tiletype {
        TileType::NonSolid => tile_flag_nonsolid,
        TileType::Solid => tile_flag_solid,
        TileType::SolidOnTop => tile_flag_solid_on_top,
        TileType::Ice => tile_flag_solid | tile_flag_ice,

        TileType::Death => tile_flag_solid | tile_flag_has_regular_death,
        TileType::DeathOnTop => tile_flag_solid | tile_flag_death_on_top,
        TileType::DeathOnBottom => tile_flag_solid | tile_flag_death_on_bottom,
        TileType::DeathOnLeft => tile_flag_solid | tile_flag_death_on_left,
        TileType::DeathOnRight => tile_flag_solid | tile_flag_death_on_right,

        TileType::IceOnTop => tile_flag_ice | tile_flag_solid_on_top,
        TileType::IceDeathOnBottom => tile_flag_solid | tile_flag_ice | tile_flag_death_on_bottom,
        TileType::IceDeathOnLleft => tile_flag_solid | tile_flag_ice | tile_flag_death_on_left,
        TileType::IceDeathOnRight => tile_flag_solid | tile_flag_ice | tile_flag_death_on_right,

        TileType::SuperDeath => tile_flag_solid | tile_flag_has_regular_death | tile_flag_has_super_death,
        TileType::SuperDeathTop => tile_flag_solid | tile_flag_super_death_top | tile_flag_death_on_top,
        TileType::SuperDeathBottom => tile_flag_solid | tile_flag_super_death_bottom | tile_flag_death_on_bottom,
        TileType::SuperDeathLeft => tile_flag_solid | tile_flag_super_death_left | tile_flag_death_on_left,
        TileType::SuperDeathRight => tile_flag_solid | tile_flag_super_death_right | tile_flag_death_on_right,
        TileType::PlayerDeath => tile_flag_player_death,

        TileType::Gap => tile_flag_gap,
        _ => tile_flag_nonsolid,
    }) as u16
}

const ordered_tile_types: [TileType; 19] = [
    TileType::NonSolid,
    TileType::Solid,
    TileType::SolidOnTop,
    TileType::Ice,
    TileType::Death,
    TileType::DeathOnTop,
    TileType::DeathOnBottom,
    TileType::DeathOnLeft,
    TileType::DeathOnRight,
    TileType::IceOnTop,
    TileType::IceDeathOnBottom,
    TileType::IceDeathOnLleft,
    TileType::IceDeathOnRight,
    TileType::SuperDeath,
    TileType::SuperDeathTop,
    TileType::SuperDeathBottom,
    TileType::SuperDeathLeft,
    TileType::SuperDeathRight,
    TileType::PlayerDeath,
];

pub fn next_tile_type(r#type: TileType) -> TileType {
    let Some(it) = ordered_tile_types.iter().position(|&t| t == r#type) else {
        return TileType::NonSolid;
    };

    let it = it + 1;
    if it == ordered_tile_types.len() {
        ordered_tile_types[0]
    } else {
        ordered_tile_types[it]
    }
}

pub fn prev_tile_type(r#type: TileType) -> TileType {
    let Some(it) = ordered_tile_types.iter().position(|&t| t == r#type) else {
        return TileType::NonSolid;
    };

    if it == 0 {
        ordered_tile_types[ordered_tile_types.len() - 1]
    } else {
        ordered_tile_types[it - 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_match_header_table() {
        assert_eq!(tile_to_flags(TileType::Death), 121);
        assert_eq!(tile_to_flags(TileType::SuperDeath), 3961);
        assert_eq!(tile_to_flags(TileType::SuperDeathRight), 2113);
        assert_eq!(tile_to_flags(TileType::IceOnTop), 6);
        assert_eq!(tile_to_flags(TileType(200)), 0);
        assert_eq!(next_tile_type(TileType::PlayerDeath), TileType::NonSolid);
        assert_eq!(prev_tile_type(TileType::NonSolid), TileType::PlayerDeath);
        assert_eq!(next_tile_type(TileType::Gap), TileType::NonSolid);
    }
}
