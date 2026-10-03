//! Port of src/smw/objects/moving/MO_PirhanaPlant.cpp

use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_drawpreview;
use crate::common::global_constants::*;
use crate::common::map::g_rPirhanaRects;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::{eyecandy, objectcontainer};
use crate::smw::main::players;
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::moving_object::{IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::objects::overmap::wo_straight_path_hazard::OMO_StraightPathHazard;
use crate::smw::player::{CPlayer, PlayerState};
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class IO_PirhanaPlant - pirhana plant that appears on a certain frequency
//------------------------------------------------------------------------------
pub struct MO_PirhanaPlant {
    pub io_moving_object: IO_MovingObject,

    pub iType: i16,
    pub iDirection: i16,
    pub iFreq: i16,
    pub iTimer: i16,
    pub iAnimationTimer: i16,
    pub iFrame: i16,
    pub iActionTimer: i16,

    pub fPreview: bool,
}
impl_base!(MO_PirhanaPlant => io_moving_object: IO_MovingObject);

impl MO_PirhanaPlant {
    pub fn new(pos: Vec2s, r#type: i16, freq: i16, direction: i16, preview: bool) -> Self {
        let mut o = MO_PirhanaPlant {
            io_moving_object: IO_MovingObject::new(Ptr::null(), pos, 1, 0, -1, -1, -1, -1, -1, -1, -1, -1),
            iType: 0,
            iDirection: 0,
            iFreq: 0,
            iTimer: 0,
            iAnimationTimer: 0,
            iFrame: 0,
            iActionTimer: 0,
            fPreview: false,
        };

        o.iType = r#type;
        o.iDirection = direction;
        o.iFreq = freq;

        o.fPreview = preview;

        o.movingObjectType = movingobject_pirhanaplant;

        o.state = 0;
        o.set_new_timer();

        if o.iDirection <= 1 {
            o.iw = 32;
        } else {
            o.ih = 32;
        }

        if direction == 0 {
            o.iy += 32;
        } else if direction == 2 {
            o.ix += 32;
        }

        if o.iDirection <= 1 {
            if o.iType == 2 {
                o.ih = 64;
            } else {
                o.ih = 48;
            }
        } else if o.iType == 2 {
            o.iw = 64;
        } else {
            o.iw = 48;
        }

        if o.iDirection <= 1 {
            o.collisionHeight = 0;
            o.collisionWidth = 32;
        } else {
            o.collisionHeight = 32;
            o.collisionWidth = 0;
        }

        o.collisionOffsetX = 0;
        o.collisionOffsetY = 0;

        o.iAnimationTimer = 0;

        o.iActionTimer = RANDOM_INT(8) as i16;
        o.iFrame = 0;

        o.fObjectCollidesWithMap = false;
        o
    }

    // For preview drawing
    pub fn draw_offset(&mut self, iOffsetX: i16, iOffsetY: i16) {
        if self.state > 0 {
            unsafe {
                let rect = &g_rPirhanaRects[self.iType as usize][self.iDirection as usize][self.iFrame as usize];
                let surface = &rm.spr_hazard_pirhanaplant[1];
                let clipRect = SDL_Rect { x: iOffsetX as i32, y: iOffsetY as i32, w: 320, h: 240 };
                let dstX = ((self.ix as i32 >> 1) + iOffsetX as i32) as i16;
                let dstY = ((self.iy as i32 >> 1) + iOffsetY as i32) as i16;
                let (ih, iw) = (self.ih as i32, self.iw as i32);
                let (ch, cw) = (self.collisionHeight as i32, self.collisionWidth as i32);
                if self.iDirection == 0 {
                    gfx_drawpreview(surface, dstX, dstY, (rect.x >> 1) as i16, (rect.y >> 1) as i16, 16, (ch >> 1) as i16, &clipRect, true, None);
                } else if self.iDirection == 1 {
                    gfx_drawpreview(surface, dstX, dstY, (rect.x >> 1) as i16, ((rect.y + ih - ch) >> 1) as i16, 16, (ch >> 1) as i16, &clipRect, true, None);
                } else if self.iDirection == 2 {
                    gfx_drawpreview(surface, dstX, dstY, (rect.x >> 1) as i16, (rect.y >> 1) as i16, (cw >> 1) as i16, 16, &clipRect, true, None);
                } else {
                    gfx_drawpreview(surface, dstX, dstY, ((rect.x + iw - cw) >> 1) as i16, (rect.y >> 1) as i16, (cw >> 1) as i16, 16, &clipRect, true, None);
                }
            }
        }
    }

    pub fn kill_plant(&mut self) {
        if self.state == 0 {
            return;
        }

        self.set_new_timer();
        self.state = 0;

        unsafe {
            if_sound_on_play(&mut rm.sfx_kicksound);
            let x = (self.ix as i32 + if self.iDirection == 2 { 0 } else { self.collisionWidth as i32 - 32 }) as i16;
            let y = (self.iy as i32 + if self.iDirection == 0 { 0 } else { self.collisionHeight as i32 - 32 }) as i16;
            eyecandy[2].emplace(EC_SingleAnimation::new(Ptr::from_mut(&mut rm.spr_fireballexplosion), x, y, 3, 4));
        }

        if self.iDirection == 0 {
            self.iy += self.collisionHeight;
        } else if self.iDirection == 2 {
            self.ix += self.collisionWidth;
        }

        if self.iDirection <= 1 {
            self.collisionHeight = 0;
        } else {
            self.collisionWidth = 0;
        }
    }

    fn set_new_timer(&mut self) {
        self.iTimer = (self.iFreq as i32 + RANDOM_INT(self.iFreq as i32)) as i16;

        // Face the green fireball plant in a random direction
        if self.iType == 0 {
            // Only point flower towards directions that make sense
            if (self.ix >> 5) == 19 {
                self.iFrame = RANDOM_INT(2) as i16;
            } else if self.ix == 0 {
                self.iFrame = (RANDOM_INT(2) + 2) as i16;
            } else {
                self.iFrame = RANDOM_INT(4) as i16;
            }
        }
    }

    fn get_fireball_angle(&self) -> f32 {
        if self.iDirection <= 1 {
            if self.iFrame == 0 {
                return -2.7214f32;
            } else if self.iFrame == 1 {
                return 2.7214f32;
            } else if self.iFrame == 2 {
                return -0.4202f32;
            } else if self.iFrame == 3 {
                return 0.4202f32;
            }
        } else if self.iFrame == 0 {
            return -1.9910f32;
        } else if self.iFrame == 1 {
            return -1.1506f32;
        } else if self.iFrame == 2 {
            return 1.9910f32;
        } else if self.iFrame == 3 {
            return 1.1506f32;
        }

        0.0f32
    }
}

impl CObjectTrait for MO_PirhanaPlant {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn update(&mut self) {
        // Needed for collisions with player and kuribo's shoe to know
        // if the plant hit the player from the top or not
        self.fOldY = self.iy as f32;

        if self.state == 0 {
            // waiting to appear
            self.iTimer -= 1;
            if self.iTimer <= 0 {
                self.iTimer = 0;
                self.state = 1;
            }
        } else if self.state == 1 {
            // appearing
            if self.iDirection <= 1 {
                self.collisionHeight += 2;
            } else {
                self.collisionWidth += 2;
            }

            if self.iDirection == 0 {
                self.iy -= 2;
            } else if self.iDirection == 2 {
                self.ix -= 2;
            }

            if (self.iDirection <= 1 && self.collisionHeight >= self.ih) || (self.iDirection >= 2 && self.collisionWidth >= self.iw) {
                self.state = 2;
            }
        } else if self.state == 2 {
            // extended
            self.iTimer += 1;
            if self.iTimer > 60 {
                self.iTimer = 0;
                self.state = 3;
            }
        } else if self.state == 3 {
            // retreating
            if self.iDirection <= 1 {
                self.collisionHeight -= 2;
            } else {
                self.collisionWidth -= 2;
            }

            if self.iDirection == 0 {
                self.iy += 2;
            } else if self.iDirection == 2 {
                self.ix += 2;
            }

            if (self.iDirection <= 1 && self.collisionHeight <= 0) || (self.iDirection >= 2 && self.collisionWidth <= 0) {
                self.state = 0;
                self.set_new_timer();
            }
        }

        if self.iType == 1 {
            // face the plant towards the nearest player
            // Don't do this every frame, just once every 8 frames
            if self.state > 0 && {
                self.iActionTimer += 1;
                self.iActionTimer >= 8
            } {
                let mut distance_to_player: i32 = App::screenWidth * 1000;
                let mut iDiffX: i16 = 1;
                let mut iDiffY: i16 = 1;

                let iPlantX: i16 = (self.ix as i32 + 16) as i16;
                let iPlantY: i16 = (self.iy as i32 + if self.iDirection == 0 { 16 } else { self.ih as i32 - 16 }) as i16;

                unsafe {
                    for i in 0..players.len() {
                        let player = players[i];
                        if player.state != PlayerState::Ready {
                            continue;
                        }

                        // Calculate normal screen distance
                        let tx: i16 = (iPlantX as i32 - player.ix as i32 - PW) as i16;
                        let ty: i16 = (iPlantY as i32 - player.iy as i32 - PH) as i16;

                        let distance_player_pow2: i32 = tx as i32 * tx as i32 + ty as i32 * ty as i32;

                        if distance_player_pow2 < distance_to_player {
                            distance_to_player = distance_player_pow2;
                            iDiffX = tx;
                            iDiffY = ty;
                        }
                    }
                }

                let dAngle: f32 = (iDiffX as f64).atan2(iDiffY as f64) as f32;

                if dAngle >= 0.0f32 && dAngle < HALF_PI {
                    self.iFrame = 0;
                } else if dAngle >= HALF_PI && dAngle <= PI {
                    self.iFrame = if self.iDirection <= 1 { 1 } else { 2 };
                } else if dAngle >= -HALF_PI && dAngle < 0.0f32 {
                    self.iFrame = if self.iDirection <= 1 { 2 } else { 1 };
                } else if dAngle >= -PI && dAngle < -HALF_PI {
                    self.iFrame = 3;
                }
            }
        } else if self.iType == 2 || self.iType == 3 {
            // Animate if these are animated plants
            self.iAnimationTimer += 1;
            if self.iAnimationTimer >= 8 {
                self.iAnimationTimer = 0;

                self.iFrame += 1;
                if self.iFrame > 1 {
                    self.iFrame = 0;
                }
            }
        }

        // Fire a fireball
        if self.iType <= 1 && self.state == 2 && self.iTimer == 30 {
            unsafe {
                let (ix, iy, iw, ih) = (self.ix as i32, self.iy as i32, self.iw as i32, self.ih as i32);
                let pos = Vec2s::new(
                    (if self.iDirection != 3 { ix + 7 } else { ix + iw - 23 }) as i16,
                    (if self.iDirection != 1 { iy + 7 } else { iy + ih - 23 }) as i16,
                );
                let angle = self.get_fireball_angle();
                objectcontainer[1].add(Ptr::new_box(OMO_StraightPathHazard::new(
                    Ptr::from_mut(&mut rm.spr_hazard_fireball[if self.fPreview { 1 } else { 0 }]),
                    pos,
                    angle,
                    3.0f32,
                    4,
                    8,
                    18,
                    18,
                    0,
                    0,
                    0,
                    if self.iFrame <= 1 { 18 } else { 0 },
                    18,
                    18,
                )));
            }
        }
    }

    fn draw(&mut self) {
        if self.state > 0 {
            unsafe {
                let rect = &g_rPirhanaRects[self.iType as usize][self.iDirection as usize][self.iFrame as usize];
                let (ix, iy) = (self.ix as i32, self.iy as i32);
                let (ih, iw) = (self.ih as i32, self.iw as i32);
                let (ch, cw) = (self.collisionHeight as i32, self.collisionWidth as i32);
                if self.iDirection == 0 {
                    rm.spr_hazard_pirhanaplant[0].draw_src(ix, iy, &SDL_Rect { x: rect.x, y: rect.y, w: 32, h: ch });
                } else if self.iDirection == 1 {
                    rm.spr_hazard_pirhanaplant[0].draw_src(ix, iy, &SDL_Rect { x: rect.x, y: rect.y + ih - ch, w: 32, h: ch });
                } else if self.iDirection == 2 {
                    rm.spr_hazard_pirhanaplant[0].draw_src(ix, iy, &SDL_Rect { x: rect.x, y: rect.y, w: cw, h: 32 });
                } else {
                    rm.spr_hazard_pirhanaplant[0].draw_src(ix, iy, &SDL_Rect { x: rect.x + iw - cw, y: rect.y, w: cw, h: 32 });
                }
            }
        }
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        if self.state == 0 {
            return false;
        }

        let fHitPlayerTop = self.fOldY + self.collisionHeight as f32 <= player.fOldY && self.iy as i32 + self.collisionHeight as i32 >= player.iy as i32;

        if player.is_invincible() || player.tanookisuit.is_statue() || (player.kuriboshoe.is_on() && !fHitPlayerTop) {
            self.kill_plant();
        } else if !player.is_shielded() && !player.shyguy {
            return player.kill_player_map_hazard(false, KillStyle::Environment, false, -1) != PlayerKillType::NonKill;
        }

        false
    }

    fn collide_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) {
        if self.state == 0 {
            return;
        }

        removeifprojectile(object, true, false);

        let r#type = object.get_moving_object_type();

        if r#type == movingobject_fireball
            || r#type == movingobject_hammer
            || r#type == movingobject_boomerang
            || r#type == movingobject_shell
            || r#type == movingobject_throwblock
            || r#type == movingobject_throwbox
            || r#type == movingobject_attackzone
            || r#type == movingobject_explosion
        {
            // Don't kill things with shells that are sitting still
            if r#type == movingobject_shell && object.get_state() == 2 {
                return;
            }

            // Don't kill things with boxesx that aren't moving fast enough
            if r#type == movingobject_throwbox && !object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity() {
                return;
            }

            if r#type == movingobject_shell || r#type == movingobject_throwblock {
                object.check_and_die();
            } else if r#type == movingobject_attackzone || r#type == movingobject_throwbox {
                object.die();
            }

            self.kill_plant();
        }
    }
}

impl IO_MovingObjectTrait for MO_PirhanaPlant {
    crate::impl_io_moving_object_plumbing!();
}
