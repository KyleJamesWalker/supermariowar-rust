//! Port of src/common/gfx/gfxPalette.cpp

use crate::globals::Aliased;
use crate::common::gfx::color::RGB;
use crate::common::gfx::get_rgb;
use crate::common::global_constants::MAX_PLAYERS;
use crate::common::gfx::gfx_sprite::ImageLoader;
use sdl2::sys::{SDL_LockSurface, SDL_UnlockSurface, SDL_RLEACCEL};
use std::collections::HashMap;
use std::path::Path;

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
pub const COUNT: PlayerPalette = 9;

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
    pub fn replacement_for(&self, teamIdx: usize, playerState: PlayerPalette) -> RGB {
        let idx = teamIdx * COUNT as usize + playerState as usize;
        self.replacements[idx]
    }
}

#[derive(Default)]
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

    pub fn replacement_for(&self, keyColor: RGB, teamIndex: usize, playerState: PlayerPalette) -> Option<RGB> {
        self.m_colorsheets.get(&keyColor).map(|sheet| sheet.replacement_for(teamIndex, playerState))
    }

    pub fn load(&mut self, path: &Path) -> bool {
        self.m_colorsheets.clear();

        let sprite = ImageLoader::new(path).without_color_key().without_optimization().create();

        unsafe {
            let surf = sprite.get_surface();
            if (*surf).flags & SDL_RLEACCEL != 0 {
                SDL_LockSurface(surf);
            }

            for x in 0..sprite.get_width() {
                let key = get_rgb(surf, x, 0);

                let mut sheet = ColorSheet::default();
                for team in 0..MAX_PLAYERS as usize {
                    for playerState in 0..COUNT as usize {
                        let relativeY = team * COUNT as usize + playerState;
                        sheet.replacements[relativeY] = get_rgb(surf, x, 1 + relativeY as i32);
                    }
                }

                // unordered_map::emplace keeps the first entry for a duplicate key
                self.m_colorsheets.entry(key).or_insert(sheet);
            }

            if (*surf).flags & SDL_RLEACCEL != 0 {
                SDL_UnlockSurface(surf);
            }
        }
        true
    }
}
