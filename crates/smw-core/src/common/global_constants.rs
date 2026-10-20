//! Game-wide constants: player and map dimensions, physics, tick-rate scaling and limits.

/// The longest string an editor settings file stores, in bytes.
pub const FILEBUFSIZE: i32 = 1024;
/// Player animation states (`sprite_state`); `_R` faces right, `_L` left.
pub const PGFX_STANDING_R: i32 = 0;
pub const PGFX_STANDING_L: i32 = 1;
pub const PGFX_RUNNING_R: i32 = 2;
pub const PGFX_RUNNING_L: i32 = 3;
pub const PGFX_JUMPING_R: i32 = 4;
pub const PGFX_JUMPING_L: i32 = 5;
pub const PGFX_STOPPING_R: i32 = 6;
pub const PGFX_STOPPING_L: i32 = 7;
pub const PGFX_DEADFLYING: i32 = 8;
pub const PGFX_DEAD: i32 = 9;
pub const PGFX_LAST: i32 = 10;
pub const PANNOUNCER_SOUND_LAST: i32 = 20;
/// Player hitbox height in pixels.
pub const PH: i32 = 25;
/// Player hitbox width in pixels.
pub const PW: i32 = 22;
pub const HALFPH: i32 = 12;
pub const HALFPW: i32 = 11;
/// Hitbox offset inside the player's 32-pixel sprite, in pixels.
pub const PHOFFSET: i32 = 4;
pub const PWOFFSET: i32 = 5;
/// Velocities are in pixels per tick and accelerations in pixels per tick squared.
pub const VELMOVING: f32 = vel(4.0);
pub const VELSLOWMOVING: f32 = vel(2.2);
pub const VELMOVING_CHICKEN: f32 = vel(2.9);
pub const VELMOVINGADD: f32 = acc(0.5);
pub const VELMOVINGADDICE: f32 = VELMOVINGADD / 4.0;
pub const VELTURBOMOVING: f32 = vel(5.5);
pub const VELJUMP: f32 = vel(9.0);
pub const VELSLOWJUMP: f32 = vel(7.0);
pub const VELTURBOJUMP: f32 = vel(10.2);
pub const VELSUPERJUMP: f32 = vel(13.0);
pub const VELPUSHBACK: f32 = vel(5.0);
pub const VELMAXBREAKBLOCK: f32 = vel(3.0);
pub const VELNOTEBLOCKBOUNCE: f32 = vel(3.0);
pub const VELNOTEBLOCKREPEL: f32 = vel(5.0);
pub const VELBLOCKBOUNCE: f32 = vel(3.0);
pub const VELBLOCKREPEL: f32 = vel(3.0);
pub const VELPOWERUPBOUNCE: f32 = vel(8.1);
pub const FIREBALLBOUNCE: f32 = vel(5.0);
pub const HAMMERTHROW: f32 = vel(8.5);
pub const VELMOVINGFRICTION: f32 = acc(0.2);
pub const VELICEFRICTION: f32 = acc(0.06);
pub const VELAIRFRICTION: f32 = acc(0.06);
pub const VELSTOPJUMP: f32 = vel(5.0);
/// The share of vertical speed kept, reversed, when bouncing off something.
pub const BOUNCESTRENGTH: f32 = 0.5;
pub const TAGGEDBOOST: f32 = vel(1.0);
pub const VELSUPERSTOMP: f32 = vel(10.0);
pub const VELTAILSHAKE: f32 = vel(1.0);
pub const VELKURIBOBOUNCE: f32 = vel(3.0);
pub const VELENEMYBOUNCE: f32 = vel(7.0);
pub const GRAVITATION: f32 = acc(0.40);
pub const MAXVELY: f32 = vel(20.0);
pub const MAXSIDEVELY: f32 = vel(10.0);
/// Screen shake from a stomp: recovery per tick, the push per stomp, and the cap, in pixels.
pub const CRUNCHVELOCITY: i32 = 2;
pub const CRUNCHAMOUNT: i32 = 16;
pub const CRUNCHMAX: i32 = 32;
/// The map size in tiles.
pub const MAPWIDTH: i32 = 20;
pub const MAPHEIGHT: i32 = 15;
/// Tile layers in a map.
pub const MAPLAYERS: i32 = 4;
/// Tile edge in pixels at full size, in the map preview, and in thumbnails.
pub const TILESIZE: i32 = 32;
pub const PREVIEWTILESIZE: i32 = 16;
pub const THUMBTILESIZE: i32 = 8;
pub const MAXWARPS: i32 = 32;
pub const MAXMAPITEMS: i32 = 32;
pub const MAXMAPHAZARDS: i32 = 30;
pub const NUMMAPHAZARDPARAMS: i32 = 5;
pub const NUMSPAWNAREATYPES: i32 = 6;
pub const MAXSPAWNAREAS: i32 = 128;
pub const MAXDRAWAREAS: i32 = 128;
pub const MAX_BONUS_CHESTS: i32 = 5;
pub const MAX_WORLD_BONUSES_AWARDED: i32 = 3;
/// Tiles in one tileset: `TILESETWIDTH` by `TILESETHEIGHT`.
pub const TILESETSIZE: i32 = 960;
pub const TILESETHEIGHT: i32 = 30;
pub const TILESETWIDTH: i32 = 32;
/// Special tileset ids a map tile can hold instead of a tileset index.
pub const TILESETANIMATED: i32 = -1;
pub const TILESETNONE: i32 = -2;
pub const TILESETUNKNOWN: i32 = -3;
/// Tiles in the animated tileset.
pub const TILEANIMATIONSIZE: i32 = 38;
pub const MAXMUSICCATEGORY: i32 = 11;
pub const MAXWORLDMUSICCATEGORY: i32 = 9;
pub const WORLDMUSICBONUS: i32 = MAXWORLDMUSICCATEGORY;
pub const WORLDMUSICSLEEP: i32 = MAXWORLDMUSICCATEGORY + 1;
pub const WORLDMUSICWORLDS: i32 = MAXWORLDMUSICCATEGORY + 2;
pub const MAXCATEGORYTRACKS: i32 = 64;
pub const MAXEYECANDY: i32 = 192;
/// How long a corpse stays on screen, in ticks.
pub const CORPSESTAY: i32 = ticks(200);
/// Ticks of game logic per second of wall time on every platform, whatever the display refreshes at. Netplay
/// depends on it, so changing it changes the protocol version.
pub const TICKS_PER_SECOND: u32 = 60;
/// The tick rate the per-tick constants are tuned for; `vel`, `acc` and `ticks` rescale them to `TICKS_PER_SECOND`
/// so the game runs at the same speed in wall time.
pub const ORIGINAL_TICKS_PER_SECOND: f32 = 62.5;
/// Tuned ticks per actual tick.
pub const TICK_SCALE: f32 = ORIGINAL_TICKS_PER_SECOND / TICKS_PER_SECOND as f32;
/// A velocity tuned per 62.5 Hz tick, in pixels per tick at `TICKS_PER_SECOND`.
pub const fn vel(per_original_tick: f32) -> f32 {
    per_original_tick * TICK_SCALE
}
/// An acceleration tuned per 62.5 Hz tick squared, rescaled to `TICKS_PER_SECOND`.
pub const fn acc(per_original_tick_squared: f32) -> f32 {
    per_original_tick_squared * TICK_SCALE * TICK_SCALE
}
/// A tick count tuned at 62.5 Hz, rescaled to `TICKS_PER_SECOND` and rounded half away from zero.
pub const fn ticks(original_ticks: i32) -> i32 {
    let (num, den) = (TICKS_PER_SECOND as i64 * 2, (ORIGINAL_TICKS_PER_SECOND * 2.0) as i64);
    let scaled = original_ticks as i64 * num;
    (if scaled >= 0 { (scaled + den / 2) / den } else { (scaled - den / 2) / den }) as i32
}
/// Milliseconds from `start_ms` to the next tick boundary of wall time, for loops that sleep in whole ms.
pub const fn ms_to_next_tick(start_ms: u32, now_ms: u32) -> u32 {
    let tps = TICKS_PER_SECOND as u64;
    let next = ((start_ms as u64 * tps / 1000 + 1) * 1000).div_ceil(tps) as u32;
    next.saturating_sub(now_ms)
}
/// A duration options files, world files and tours store in 62 Hz frames, as ticks, rounded to nearest.
pub const fn ticks_from_disk(frames: i16) -> i16 {
    ((frames as i32 * TICKS_PER_SECOND as i32 + 31) / 62) as i16
}
/// A duration in ticks as 62 Hz frames for writing to disk; the inverse of `ticks_from_disk`.
pub const fn ticks_to_disk(ticks: i16) -> i16 {
    ((ticks as i32 * 62 + TICKS_PER_SECOND as i32 / 2) / TICKS_PER_SECOND as i32) as i16
}
/// The Graphics Options "Frame Rate" setting: present at the display's refresh, or at most one of these rates. Older
/// options files hold a frame time in ms (at most 100) in its byte, so any other value reads as Display.
pub const FRAME_RATE_DISPLAY: i16 = 200;
/// The "Frame Rate" setting codes and the frames per second each caps at.
pub const FRAME_RATE_CAPS: [(i16, u32); 7] = [(201, 30), (202, 40), (203, 45), (204, 60), (205, 90), (206, 120), (207, 144)];

/// The frame rate cap `setting` names, or `None` for Display and any unknown value.
pub fn frame_rate_cap(setting: i16) -> Option<u32> {
    FRAME_RATE_CAPS.iter().find(|&&(code, _)| code == setting).map(|&(_, fps)| fps)
}
/// The most awards a kill streak can show.
pub const MAXAWARDS: i32 = 10;
/// Kills in a row before the first award.
pub const MINAWARDSNEEDED: i32 = 3;
pub const MAXTOURNAMENTGAMES: i32 = 10;
pub const MAXWORLDITEMS: i32 = 32;
pub const MAXRACEGOALS: i32 = 8;
pub const TWO_PI: f32 = 6.2831853;
/// Not quite π; changing it changes gameplay and the replay goldens.
pub const PI: f32 = 3.1415292;
pub const THREE_HALF_PI: f32 = 4.7123890;
pub const THREE_QUARTER_PI: f32 = 2.3561945;
pub const HALF_PI: f32 = 1.5707963;
pub const QUARTER_PI: f32 = 0.7853982;
/// Powerup kinds a block or item can hold.
pub const NUM_POWERUPS: i32 = 26;
/// Powerups only the world map offers.
pub const NUM_WORLD_POWERUPS: i32 = 15;
pub const NUM_WORLD_SCORE_BONUSES: i32 = 20;
/// Every kind of world map item: powerups, world powerups and score bonuses.
pub const NUM_WORLD_ITEMS: i32 = NUM_POWERUPS + NUM_WORLD_POWERUPS + NUM_WORLD_SCORE_BONUSES;
/// Settings per powerup block: one weight per powerup.
pub const NUM_BLOCK_SETTINGS: i32 = NUM_POWERUPS;
/// Built-in powerup weight presets.
pub const NUM_POWERUP_PRESETS: i32 = 17;
/// Pseudo powerup ids below zero, for items no block holds.
pub const HEALTH_POWERUP: i32 = -1;
pub const TIME_POWERUP: i32 = -2;
pub const JAIL_KEY_POWERUP: i32 = -3;
pub const COIN_POWERUP: i32 = -4;
pub const MINIGAME_COIN: i32 = -5;
pub const SECRET1_POWERUP: i32 = -6;
pub const SECRET2_POWERUP: i32 = -7;
pub const SECRET3_POWERUP: i32 = -8;
pub const SECRET4_POWERUP: i32 = -9;
pub const NO_POWERUP: i32 = -10;
pub const NUM_AUTO_FILTERS: i32 = 12;
pub const NUMSTOMPENEMIES: i32 = 9;
pub const NUMSURVIVALENEMIES: i32 = 3;
pub const NUMFRENZYCARDS: i32 = 19;
pub const MAX_PLAYERS: i32 = 4;

#[cfg(test)]
mod tick_scale_tests {
    use super::*;

    #[test]
    fn ticks_keep_wall_time() {
        assert_eq!(ticks(25), 24);
        assert_eq!(ticks(580), 557);
        assert_eq!(ticks(4), 4);
        assert_eq!(ticks(-25), -24);
        assert_eq!(ticks_from_disk(1860), 1800);
        assert_eq!(ticks_to_disk(1800), 1860);
        assert_eq!(ms_to_next_tick(0, 0), 17);
        assert_eq!(ms_to_next_tick(17, 20), 14);
    }
}
