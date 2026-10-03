//! Port of src/smw/objects/moving/MO_Boomerang.cpp

use crate::common::eyecandy::Spotlight;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::{BoomerangStyle, TeamCollisionStyle};
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::spotlightManager;
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::blocks::weapon_breakable_block::{B_WeaponBreakableBlock, WeaponDamageType};
use crate::smw::objects::moving::moving_object::{IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::{get_player_from_global_id, player_killed_player, CPlayer, PlayerDeathStyle};
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class boomerang
//------------------------------------------------------------------------------
pub struct MO_Boomerang {
    pub io_moving_object: IO_MovingObject,

    pub colorOffset: i16,

    pub fMoveToRight: bool,
    pub fFlipped: bool,
    pub iStateTimer: i16,

    pub iStyle: BoomerangStyle,
    pub sSpotlight: Ptr<Spotlight>,
}
impl_base!(MO_Boomerang => io_moving_object: IO_MovingObject);

impl MO_Boomerang {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, iNumSpr: i16, moveToRight: bool, aniSpeed: i16, iGlobalID: i16, teamID: i16, iColorID: i16) -> Self {
        let mut o = MO_Boomerang {
            io_moving_object: IO_MovingObject::new(
                nspr,
                pos,
                iNumSpr,
                aniSpeed,
                (nspr.get_width() as i16 as i32 / iNumSpr as i32) as i16,
                (nspr.get_height() as i16) >> 3,
                0,
                0,
                -1,
                -1,
                -1,
                -1,
            ),
            colorOffset: 0,
            fMoveToRight: false,
            fFlipped: false,
            iStateTimer: 0,
            iStyle: BoomerangStyle::Flat,
            sSpotlight: Ptr::null(),
        };

        // boomerangs sprites have both right and left sprites in them
        o.ih = o.ih >> 3;

        o.iPlayerID = iGlobalID;
        o.iTeamID = teamID;
        o.colorOffset = (iColorID as i32 * 64) as i16;
        o.movingObjectType = movingobject_boomerang;

        o.state = 1;

        o.fMoveToRight = moveToRight;

        if moveToRight {
            o.velx = 5.0;
        } else {
            o.velx = -5.0;
        }

        o.vely = 0.0;

        o.fFlipped = false;

        // Don't let boomerang start off the screen or it won't rebound correctly
        if moveToRight && o.fx + o.iw as f32 >= App::screenWidth as f32 {
            let fx = o.fx - App::screenWidth as f32;
            o.set_xf(fx);
        } else if !moveToRight && o.fx < 0.0 {
            let fx = o.fx + App::screenWidth as f32;
            o.set_xf(fx);
        }

        o.iStateTimer = 0;

        unsafe {
            if game_values.boomerangstyle == BoomerangStyle::Random {
                o.iStyle = BoomerangStyle::from_u8(RANDOM_INT(3) as u8);
            } else {
                o.iStyle = game_values.boomerangstyle;
            }
        }

        o.fObjectCollidesWithMap = false;

        o.sSpotlight = Ptr::null();
        o
    }

    // Call to kill boomerang when it is not caught by player
    fn forcedead(&mut self) {
        unsafe {
            removeifprojectile(self.as_mo_ptr(), false, true);
            rm.sfx_boomerang.stop();

            if game_values.boomeranglimit == 0 {
                return;
            }

            // Penalize player if they did not catch it
            let mut player = get_player_from_global_id(self.iPlayerID);

            if !player.is_null() {
                if player.projectilelimit > 0 {
                    player.decrease_projectile_limit();
                }
            }
        }
    }
}

impl CObjectTrait for MO_Boomerang {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn update(&mut self) {
        unsafe {
            if !rm.sfx_boomerang.is_playing() {
                if_sound_on_play(&mut rm.sfx_boomerang);
            }

            self.animate();

            // Detection collision with boomerang breakable blocks
            let blocks = self.get_collision_blocks();
            for iBlock in 0..4usize {
                let mut block = blocks[iBlock];
                if block.is_null() {
                    continue;
                }
                if let Some(weaponbreakableblock) = block.as_any().downcast_mut::<B_WeaponBreakableBlock>() {
                    if weaponbreakableblock.r#type() == WeaponDamageType::Boomerang {
                        weaponbreakableblock.trigger_behavior_player(self.iPlayerID, self.iTeamID);
                        self.forcedead();
                        return;
                    }
                }
            }

            let screenWidth = App::screenWidth as f32;

            if self.iStyle == BoomerangStyle::Flat {
                // Flat style
                self.fOldX = self.fx;
                let fx = self.fx + self.velx;
                self.set_xf(fx);

                if self.fMoveToRight && self.fx + self.iw as f32 >= screenWidth && self.fOldX + (self.iw as f32) < screenWidth {
                    if self.fFlipped {
                        self.forcedead();
                        return;
                    } else {
                        let fx = (App::screenWidth - self.iw as i32) as f32;
                        self.set_xf(fx);
                        self.fFlipped = true;
                        self.fMoveToRight = false;
                        self.velx = -self.velx;
                    }
                } else if !self.fMoveToRight && self.fx < 0.0 && self.fOldX >= 0.0 {
                    if self.fFlipped {
                        self.forcedead();
                        return;
                    } else {
                        self.set_xf(0.0);
                        self.fFlipped = true;
                        self.fMoveToRight = true;
                        self.velx = -self.velx;
                    }
                }
            } else if self.iStyle == BoomerangStyle::SMB3 {
                // Attempting to emulate the SMB3 boomerang behavior
                self.iStateTimer += 1;

                self.fOldX = self.fx;
                let fx = self.fx + self.velx;
                self.set_xf(fx);

                if self.fx < 0.0 {
                    let fx = self.fx + screenWidth;
                    self.set_xf(fx);
                } else if self.fx + self.iw as f32 >= screenWidth {
                    let fx = self.fx - screenWidth;
                    self.set_xf(fx);
                }

                if self.state == 1 {
                    self.fOldY = self.fy;
                    let fy = self.fy - 3.2f32;
                    self.set_yf(fy);

                    if self.iStateTimer >= 20 {
                        self.iStateTimer = 0;
                        self.state = 2;
                    }
                } else if self.state == 2 {
                    if self.iStateTimer >= 26 {
                        self.iStateTimer = 0;
                        self.state = 3;
                    }
                } else if self.state == 3 {
                    self.fOldY = self.fy;
                    let fy = self.fy + 1.0f32;
                    self.set_yf(fy);

                    if self.fMoveToRight {
                        // Add amount so that by time fy lowers by two tiles, we turn around the boomerang
                        self.velx -= 0.15625f32;

                        if self.velx <= -5.0f32 {
                            self.velx = -5.0;
                            self.state = 4;
                            self.fFlipped = true;
                        }
                    } else {
                        self.velx += 0.15625f32;

                        if self.velx >= 5.0f32 {
                            self.velx = 5.0;
                            self.state = 4;
                            self.fFlipped = true;
                        }
                    }

                    self.iStateTimer = 0;
                } else if self.state == 4 {
                    if self.iStateTimer >= 46 {
                        if (self.fMoveToRight && self.fx < 0.0 && self.fOldX >= 0.0)
                            || (!self.fMoveToRight && self.fx + self.iw as f32 >= screenWidth && self.fOldX + (self.iw as f32) < screenWidth)
                        {
                            self.forcedead();
                            return;
                        }
                    }
                }
            } else if self.iStyle == BoomerangStyle::Zelda {
                // Zelda style boomerang
                self.iStateTimer += 1;

                self.fOldX = self.fx;
                let fx = self.fx + self.velx;
                self.set_xf(fx);

                if self.fx < 0.0 {
                    let fx = self.fx + screenWidth;
                    self.set_xf(fx);
                } else if self.fx + self.iw as f32 >= screenWidth {
                    let fx = self.fx - screenWidth;
                    self.set_xf(fx);
                }

                if self.iStateTimer > game_values.boomeranglife {
                    self.forcedead();
                    return;
                }

                if self.state == 1 {
                    if self.iStateTimer >= 64 {
                        self.state = 2;
                        self.fFlipped = true;

                        /*
                        CPlayer * player = GetPlayerFromGlobalID(iPlayerID);

                        //No wrap boomerang
                        if (player)
                        {
                            if ((player->ix < ix && velx > 0) || (player->ix > ix && velx < 0))
                                        velx = -velx;
                        }
                        else
                        {
                                velx = -velx;
                        }
                        */

                        self.velx = -self.velx; // Wrap Boomerang
                    }
                } else if self.state == 2 {
                    self.fOldY = self.fy;
                    let fy = self.fy + self.vely;
                    self.set_yf(fy);

                    // Follow the player zelda style
                    let player = get_player_from_global_id(self.iPlayerID);

                    if !player.is_null() {
                        let mut fWrap = false;
                        if (player.ix as i32 - self.ix as i32).abs() > 320 {
                            fWrap = true;
                        }

                        if (player.ix < self.ix && !fWrap) || (player.ix > self.ix && fWrap)
                        // Wrap Boomerang
                        // if (player->ix < ix)  //No Wrap Boomerang
                        {
                            self.velx -= 0.2f32;

                            if self.velx < -5.0f32 {
                                self.velx = -5.0;
                            }
                        } else {
                            self.velx += 0.2f32;

                            if self.velx > 5.0f32 {
                                self.velx = 5.0;
                            }
                        }

                        if player.iy < self.iy {
                            self.vely -= 0.2f32;

                            if self.vely < -3.0f32 {
                                self.vely = -3.0;
                            }
                        } else {
                            self.vely += 0.2f32;

                            if self.vely > 3.0f32 {
                                self.vely = 3.0;
                            }
                        }
                    } else {
                        // Remove boomerang if player was removed from game
                        self.forcedead();
                        return;

                        /*
                        //Die at nearest edge if player was removed from game
                        if (velx > 0)
                                velx = 5.0f;
                        else
                                velx = -5.0f;

                        if ((fx < 0.0f && fOldX >= 0.0f) ||
                                (fx + iw >= App::screenWidth && fOldX + iw < App::screenWidth))
                        {
                                forcedead();
                                return;
                        }
                        */
                    }
                }
            }

            if game_values.spotlights {
                if self.sSpotlight.is_null() {
                    self.sSpotlight = spotlightManager.add_spotlight(
                        (self.ix as i32 - self.collisionOffsetX as i32 + (self.iw as i32 >> 1)) as i16,
                        (self.iy as i32 - self.collisionOffsetY as i32 + (self.ih as i32 >> 1)) as i16,
                        3,
                    );
                }

                if !self.sSpotlight.is_null() {
                    let x = (self.ix as i32 - self.collisionOffsetX as i32 + (self.iw as i32 >> 1)) as i16;
                    let y = (self.iy as i32 - self.collisionOffsetY as i32 + (self.ih as i32 >> 1)) as i16;
                    self.sSpotlight.update_position(x, y);
                }
            }
        }
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        unsafe {
            if self.iPlayerID != player.globalID && (game_values.teamcollision == TeamCollisionStyle::On || self.iTeamID != player.teamID) {
                if !player.is_shielded() {
                    removeifprojectile(self.as_mo_ptr(), false, false);

                    if !player.is_invincible() && !player.shyguy {
                        // Find the player that shot this boomerang so we can attribute a kill
                        player_killed_player(self.iPlayerID, player, PlayerDeathStyle::Jump, KillStyle::Boomerang, false, false);
                        return true;
                    }
                }
            } else if self.iPlayerID == player.globalID && self.fFlipped {
                removeifprojectile(self.as_mo_ptr(), false, true);
            }
        }

        false
    }

    fn draw(&mut self) {
        self.spr.draw_src(
            self.ix as i32 - self.collisionOffsetX as i32,
            self.iy as i32 - self.collisionOffsetY as i32,
            &SDL_Rect {
                x: self.drawframe as i32,
                y: self.colorOffset as i32 + if self.fMoveToRight { 0 } else { 32 },
                w: self.iw as i32,
                h: self.ih as i32,
            },
        );
    }
}

impl IO_MovingObjectTrait for MO_Boomerang {
    crate::impl_io_moving_object_plumbing!();
}
