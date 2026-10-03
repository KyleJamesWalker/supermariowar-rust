//! Port of src/smw/objects/blocks/ViewBlock.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::io_block::IO_BlockTrait;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::{rm, Ptr};
use crate::impl_base;
use crate::smw::objects::blocks::powerup_block::*;
use sdl2::sys::SDL_Rect;

pub struct B_ViewBlock {
    pub b_powerup_block: B_PowerupBlock,

    pub poweruptimer: i16,
    pub powerupindex: i16,

    pub fNoPowerupsSelected: bool,
    pub iCountWeight: i16,
}
impl_base!(B_ViewBlock => b_powerup_block: B_PowerupBlock);

impl B_ViewBlock {
    pub fn new(nspr1: Ptr<gfxSprite>, pos: Vec2s, fHidden: bool, piSettings: &[i16]) -> Self {
        let mut b = B_ViewBlock {
            b_powerup_block: B_PowerupBlock::new(nspr1, pos, 1, 32000, fHidden, piSettings),
            poweruptimer: 0,
            powerupindex: 0,
            fNoPowerupsSelected: false,
            iCountWeight: 0,
        };

        b.poweruptimer = 0;
        b.powerupindex = RANDOM_INT(NUM_POWERUPS) as i16;

        b.iw = 32;
        b.ih = 32;

        b.iCountWeight = 0;
        for iPowerup in 0..NUM_POWERUPS as usize {
            b.iCountWeight = (b.iCountWeight as i32 + b.settings[iPowerup] as i32) as i16;
        }

        b.fNoPowerupsSelected = b.iCountWeight == 0;
        b.get_next_powerup();
        b
    }

    fn get_next_powerup(&mut self) {
        if self.fNoPowerupsSelected {
            return;
        }

        let iRandPowerup: i32 = RANDOM_INT(self.iCountWeight as i32) + 1;
        self.powerupindex = 0;
        let mut iPowerupWeightCount: i32 = self.settings[self.powerupindex as usize] as i32;

        while iPowerupWeightCount < iRandPowerup {
            self.powerupindex += 1;
            iPowerupWeightCount += self.settings[self.powerupindex as usize] as i32;
        }
    }
}

impl CObjectTrait for B_ViewBlock {
    crate::impl_cobject_plumbing!();
    fn as_io_block(&mut self) -> Option<&mut dyn IO_BlockTrait> {
        Some(self)
    }

    fn draw(&mut self) {
        if self.hidden {
            return;
        }

        //Draw powerup behind block
        if self.state == 0 && !self.fNoPowerupsSelected {
            unsafe {
                rm.spr_storedpoweruplarge.draw_src(
                    self.ix as i32,
                    self.iy as i32,
                    &SDL_Rect { x: self.powerupindex as i32 * 32, y: 0, w: 32, h: 32 },
                );
            }
        }

        b_powerup_block_draw(self);
    }

    fn update(&mut self) {
        b_powerup_block_update(self);

        if self.state == 0 && !self.fNoPowerupsSelected {
            self.poweruptimer += 1;
            if self.poweruptimer as i32 > self.settings[self.powerupindex as usize] as i32 * 10 {
                self.poweruptimer = 0;
                self.get_next_powerup();
            }
        }
    }
}

impl IO_BlockTrait for B_ViewBlock {
    crate::impl_io_block_plumbing!();
    crate::impl_powerup_block_io_block_overrides!();
}

impl B_PowerupBlockTrait for B_ViewBlock {
    crate::impl_powerup_block_plumbing!();

    fn select_powerup(&mut self) -> i16 {
        if self.fNoPowerupsSelected {
            return NO_POWERUP as i16;
        }

        self.powerupindex
    }
}
