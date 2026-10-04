//! Port of src/smw/objects/moving/MO_FrenzyCard.cpp

use crate::common::game::App;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::NUMFRENZYCARDS;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{object_frenzycard, CObjectTrait};
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::frenzy::CGM_Frenzy;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::objects::carriable::co_shell::{CO_Shell, ShellType};
use crate::smw::objects::moving::moving_object::{io_moving_object_draw, IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class frenzycard (for fire frenzy mode)
//------------------------------------------------------------------------------
pub struct MO_FrenzyCard {
    pub io_moving_object: IO_MovingObject,

    pub timer: i16,
    pub r#type: i16,

    pub sparkleanimationtimer: i16,
    pub sparkledrawframe: i16,
}
impl_base!(MO_FrenzyCard => io_moving_object: IO_MovingObject);

impl MO_FrenzyCard {
    pub fn new(nspr: Ptr<gfxSprite>, iType: i16) -> Self {
        let mut o = MO_FrenzyCard {
            io_moving_object: IO_MovingObject::new(nspr, Vec2s::zero(), 12, 8, -1, -1, -1, -1, 0, (iType as i32 * 32) as i16, 32, 32),
            timer: 0,
            r#type: 0,
            sparkleanimationtimer: 0,
            sparkledrawframe: 0,
        };

        o.state = 1;
        o.objectType = object_frenzycard;
        o.r#type = iType;

        if o.r#type as i32 == NUMFRENZYCARDS - 1 {
            o.r#type = RANDOM_INT(NUMFRENZYCARDS - 1) as i16;
        }

        o.sparkleanimationtimer = 0;
        o.sparkledrawframe = 0;

        o.place_card();

        o.fObjectCollidesWithMap = false;
        o
    }

    pub fn place_card(&mut self) {
        if crate::smw::net_random::place_event(self.iNetworkID, 0) {
            return;
        }

        unsafe {
            self.timer = 0;

            let mut x: i16 = 0;
            let mut y: i16 = 0;
            let mut iAttempts: i16 = 32;
            let (cw, ch) = (self.collisionWidth, self.collisionHeight);
            while (!g_map.findspawnpoint(5, &mut x, &mut y, cw, ch, false) || objectcontainer[1].get_closest_object(x, y, object_frenzycard) <= 150.0f32) && {
                let a = iAttempts;
                iAttempts -= 1;
                a > 0
            } {}

            self.set_xi(x);
            self.set_yi(y);
        }
    }
}

impl CObjectTrait for MO_FrenzyCard {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        unsafe {
            let r#type = self.r#type;
            if r#type < 14 || r#type > 17 || game_values.gamemodesettings.frenzy.storedshells {
                player.set_powerup(r#type);
                // In the boss minigame the C++ static_cast writes this past the end of the non-Frenzy mode object.
                if let Some(frenzy) = game_values.gamemode.as_any().downcast_mut::<CGM_Frenzy>() {
                    frenzy.set_frenzy_owner(player);
                }
            } else {
                match r#type {
                    14 => {
                        let mut shell = Ptr::new_box(CO_Shell::new(ShellType::Green, Vec2s::zero(), true, true, true, false));
                        if objectcontainer[1].add(shell) {
                            shell.used_as_stored_powerup(player);
                        }
                    }
                    15 => {
                        let mut shell = Ptr::new_box(CO_Shell::new(ShellType::Red, Vec2s::zero(), false, true, true, false));
                        if objectcontainer[1].add(shell) {
                            shell.used_as_stored_powerup(player);
                        }
                    }
                    16 => {
                        let mut shell = Ptr::new_box(CO_Shell::new(ShellType::Spiny, Vec2s::zero(), false, false, true, true));
                        if objectcontainer[1].add(shell) {
                            shell.used_as_stored_powerup(player);
                        }
                    }
                    17 => {
                        let mut shell = Ptr::new_box(CO_Shell::new(ShellType::Buzzy, Vec2s::zero(), false, true, false, false));
                        if objectcontainer[1].add(shell) {
                            shell.used_as_stored_powerup(player);
                        }
                    }
                    _ => {}
                }
            }

            self.dead = true;
            false
        }
    }

    fn update(&mut self) {
        self.animate();

        self.sparkleanimationtimer += 1;
        if self.sparkleanimationtimer >= 4 {
            self.sparkleanimationtimer = 0;
            self.sparkledrawframe += 32;
            if self.sparkledrawframe as i32 >= App::screenHeight {
                self.sparkledrawframe = 0;
            }
        }

        self.timer += 1;
        if self.timer > 1500 {
            self.place_card();
        }
    }

    fn draw(&mut self) {
        io_moving_object_draw(self);

        // Draw sparkles
        unsafe {
            rm.spr_shinesparkle.draw_src(
                self.ix as i32 - self.collisionOffsetX as i32,
                self.iy as i32 - self.collisionOffsetY as i32,
                &SDL_Rect { x: self.sparkledrawframe as i32, y: 0, w: 32, h: 32 },
            );
        }
    }
}

impl IO_MovingObjectTrait for MO_FrenzyCard {
    crate::impl_io_moving_object_plumbing!();
}
