//! Player color palettes: per-team and per-state replacements for the key colors in player sprites.

use crate::game::Game;
use crate::globals::Aliased;
use crate::common::gfx::color::RGB;
use crate::common::gfx::get_rgb;
use crate::common::global_constants::MAX_PLAYERS;
use crate::common::gfx::gfx_sprite::ImageLoader;
use std::collections::HashMap;
use std::path::Path;

/// Row within a team's block of a palette image: the player state whose colors to use.
pub type PlayerPalette = i32;
pub const normal: PlayerPalette = 0;
pub const invincibility_1: PlayerPalette = 1;
pub const invincibility_2: PlayerPalette = 2;
pub const invincibility_3: PlayerPalette = 3;
pub const shielded: PlayerPalette = 4;
pub const tagged: PlayerPalette = 5;
pub const ztarred: PlayerPalette = 6;
pub const got_shine: PlayerPalette = 7;
pub const frozen: PlayerPalette = 8;
/// Number of player palette states per team.
pub const COUNT: PlayerPalette = 9;

/// Replacement colors for one key color, indexed by `team * COUNT + state`.
#[derive(Clone, Copy)]
pub struct ColorSheet {
    pub replacements: [RGB; MAX_PLAYERS as usize * COUNT as usize],
}

impl Default for ColorSheet {
    fn default() -> Self {
        ColorSheet { replacements: [RGB::default(); MAX_PLAYERS as usize * COUNT as usize] }
    }
}

impl ColorSheet {
    /// Replacement color for team `teamIdx` (0 to `MAX_PLAYERS - 1`) in `playerState`.
    pub fn replacement_for(&self, teamIdx: usize, playerState: PlayerPalette) -> RGB {
        let idx = teamIdx * COUNT as usize + playerState as usize;
        self.replacements[idx]
    }
}

/// Color sheets keyed by the sprite color they replace.
#[derive(Clone, Default)]
pub struct gfxPalette {
    m_colorsheets: HashMap<RGB, ColorSheet>,
    pub _alias: Aliased,
}

impl gfxPalette {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn color_sheets(&self) -> &HashMap<RGB, ColorSheet> {
        &self.m_colorsheets
    }

    /// Replacement for `keyColor`, or `None` when the palette does not recolor it.
    pub fn replacement_for(&self, keyColor: RGB, teamIndex: usize, playerState: PlayerPalette) -> Option<RGB> {
        self.m_colorsheets.get(&keyColor).map(|sheet| sheet.replacement_for(teamIndex, playerState))
    }

    /// Loads a palette image: row 0 holds key colors, and below each key its column holds `MAX_PLAYERS * COUNT`
    /// replacements in [`ColorSheet`] order. Replaces any loaded palette and always returns `true`.
    pub fn load(&mut self, g: &mut Game, path: &Path) -> bool {
        let mut gp = g.ptr();
        self.m_colorsheets.clear();

        let sprite = ImageLoader::new(path).without_color_key().without_optimization().create(gp.get());

        let surf = sprite.get_surface();
        for x in 0..sprite.get_width() {
            let key = get_rgb(surf, x, 0);

            let mut sheet = ColorSheet::default();
            for team in 0..MAX_PLAYERS as usize {
                for playerState in 0..COUNT as usize {
                    let relativeY = team * COUNT as usize + playerState;
                    sheet.replacements[relativeY] = get_rgb(surf, x, 1 + relativeY as i32);
                }
            }

            // The first column with a given key color wins.
            self.m_colorsheets.entry(key).or_insert(sheet);
        }
        true
    }
}
