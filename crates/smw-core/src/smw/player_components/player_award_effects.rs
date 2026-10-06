//! Port of src/smw/player_components/PlayerAwardEffects.cpp

use crate::common::eyecandy::{EC_ExplodingAward, EC_FallingObject, EC_GravText, EC_RocketAward, EC_SoulsAward, EC_SwirlingAward};
use crate::common::eyecandy_styles::AwardStyle;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::ClipEdge;
use crate::common::global_constants::*;
use crate::common::math::trig::{cosf, sinf};
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::objectgame::check_secret;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

struct STextAward {
    name: &'static str,
    large: bool,
}

const PAWARD_LAST: i16 = 9;
const awardtexts: [STextAward; PAWARD_LAST as usize] = [
    STextAward { name: "Double Kill", large: false },
    STextAward { name: "Triple Kill", large: false },
    STextAward { name: "Killing Spree", large: false },
    STextAward { name: "Killing Spree x 2", large: false },
    STextAward { name: "Killing Spree x 3", large: false },
    STextAward { name: "Dominating", large: true },
    STextAward { name: "Dominating x 2", large: true },
    STextAward { name: "Dominating x 3", large: true },
    STextAward { name: "Unstoppable!", large: true },
];

#[derive(Default)]
pub struct PlayerAwardEffects {
    pub awardangle: f32,
    pub awards: [i16; MAXAWARDS as usize],
    pub _alias: Aliased,
}

impl PlayerAwardEffects {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn draw_ring_award(&mut self, player: &mut CPlayer) {
        unsafe {
            self.awardangle += 0.02f32;

            if self.awardangle > TWO_PI {
                self.awardangle -= TWO_PI;
            }

            let numawards: i16 = if player.killsinrow as i32 > MAXAWARDS { MAXAWARDS as i16 } else { player.killsinrow };
            let addangle: f32 = TWO_PI / numawards as f32;

            let xoffset: i16 = (player.center_x() as i32 - 8) as i16;
            let yoffset: i16 = (player.center_y() as i32 - 8) as i16;

            for k in 0..numawards {
                let angle: f32 = k as f32 * addangle + self.awardangle;
                let awardx: i16 = (xoffset as i32 + (30.0f32 * cosf(angle)) as i16 as i32) as i16;
                let awardy: i16 = (yoffset as i32 + (30.0f32 * sinf(angle)) as i16 as i32) as i16;

                let src = SDL_Rect { x: self.awards[k as usize] as i32 * 16, y: 0, w: 16, h: 16 };
                if player.iswarping() {
                    let edge = std::mem::transmute::<i32, ClipEdge>((player.state as i16 % 4) as i32);
                    rm.spr_award.draw_clip(awardx as i32, awardy as i32, &src, edge, player.get_warp_plane() as i32);
                } else {
                    rm.spr_award.draw_src(awardx as i32, awardy as i32, &src);
                }
            }
        }
    }

    fn add_exploding(&mut self, player: &mut CPlayer) {
        unsafe {
            if (player.killsinrow as i32) < MINAWARDSNEEDED {
                return;
            }

            if_sound_on_play(&mut rm.sfx_cannon);

            let numawards: i16 = if player.killsinrow as i32 > MAXAWARDS { MAXAWARDS as i16 } else { player.killsinrow };
            let addangle: f32 = TWO_PI / numawards as f32;

            for k in 0..numawards {
                let angle: f32 = k as f32 * addangle + self.awardangle;
                let cosangle: f32 = cosf(angle);
                let sinangle: f32 = sinf(angle);

                let awardx: i16 = (player.center_x() as i32 - 8 + (30.0f32 * cosangle) as i16 as i32) as i16;
                let awardy: i16 = (player.center_y() as i32 - 8 + (30.0f32 * sinangle) as i16 as i32) as i16;

                let awardvelx: f32 = 7.0f32 * cosangle;
                let awardvely: f32 = 7.0f32 * sinangle;

                eyecandy[2].emplace(EC_ExplodingAward::new(
                    Ptr::from_mut(&mut rm.spr_awardsolid),
                    awardx,
                    awardy,
                    awardvelx,
                    awardvely,
                    30,
                    self.awards[k as usize],
                ));
            }
        }
    }

    fn add_swirling(&mut self, player: &mut CPlayer) {
        unsafe {
            if (player.killsinrow as i32) < MINAWARDSNEEDED {
                return;
            }

            if_sound_on_play(&mut rm.sfx_cannon);

            let numawards: i16 = if player.killsinrow as i32 > MAXAWARDS { MAXAWARDS as i16 } else { player.killsinrow };
            let addangle: f32 = TWO_PI / numawards as f32;

            for k in 0..numawards {
                let angle: f32 = k as f32 * addangle + self.awardangle;

                if numawards as i32 == MAXAWARDS {
                    eyecandy[2].emplace(EC_SwirlingAward::new(
                        Ptr::from_mut(&mut rm.spr_awardkillsinrow),
                        (player.center_x() as i32 - 8) as i16,
                        (player.center_y() as i32 - 8) as i16,
                        angle,
                        30.0f32,
                        0.05f32,
                        60,
                        10,
                        player.get_color_id(),
                        16,
                        16,
                        4,
                        4,
                    ));
                } else {
                    eyecandy[2].emplace(EC_SwirlingAward::new(
                        Ptr::from_mut(&mut rm.spr_awardkillsinrow),
                        (player.center_x() as i32 - 8) as i16,
                        (player.center_y() as i32 - 8) as i16,
                        angle,
                        30.0f32,
                        0.05f32,
                        60,
                        (numawards as i32 - 1) as i16,
                        player.get_color_id(),
                        16,
                        16,
                        0,
                        0,
                    ));
                }
            }
        }
    }

    fn add_rocket(&mut self, player: &mut CPlayer) {
        unsafe {
            if (player.killsinrow as i32) < MINAWARDSNEEDED {
                return;
            }

            if_sound_on_play(&mut rm.sfx_cannon);

            let numawards: i16 = if player.killsinrow as i32 > MAXAWARDS { MAXAWARDS as i16 } else { player.killsinrow };

            let addangle: f32 = QUARTER_PI / 10.0f32;
            let startangle: f32 = -HALF_PI - ((addangle / 2.0f32) * (numawards as i32 - 1) as f32);

            for k in 0..numawards {
                let angle: f32 = k as f32 * addangle + startangle;
                let awardvelx: f32 = 9.0f32 * cosf(angle);
                let awardvely: f32 = 9.0f32 * sinf(angle);

                if numawards as i32 == MAXAWARDS {
                    eyecandy[2].emplace(EC_RocketAward::new(
                        Ptr::from_mut(&mut rm.spr_awardkillsinrow),
                        (player.center_x() as i32 - 8) as i16,
                        (player.center_y() as i32 - 8) as i16,
                        awardvelx,
                        awardvely,
                        80,
                        10,
                        player.get_color_id(),
                        16,
                        16,
                        4,
                        4,
                    ));
                } else {
                    eyecandy[2].emplace(EC_RocketAward::new(
                        Ptr::from_mut(&mut rm.spr_awardkillsinrow),
                        (player.center_x() as i32 - 8) as i16,
                        (player.center_y() as i32 - 8) as i16,
                        awardvelx,
                        awardvely,
                        80,
                        (numawards as i32 - 1) as i16,
                        player.get_color_id(),
                        16,
                        16,
                        0,
                        0,
                    ));
                }
            }
        }
    }

    pub fn add_death_award(&mut self, player: &mut CPlayer) {
        unsafe {
            if game_values.awardstyle == AwardStyle::Halo {
                self.add_exploding(player);
            } else if game_values.awardstyle == AwardStyle::Souls && player.killsinrow as i32 >= MINAWARDSNEEDED {
                eyecandy[2].emplace(EC_SoulsAward::new(
                    Ptr::from_mut(&mut rm.spr_awardsouls),
                    Ptr::from_mut(&mut rm.spr_awardsoulspawn),
                    player.center_x(),
                    player.center_y(),
                    60,
                    9.0f32,
                    player.killsinrow,
                    &self.awards,
                ));
            }

            player.killsinrow = 0;
            player.killsinrowinair = 0;
        }
    }

    pub fn add_killer_award(&mut self, killer: &mut CPlayer, killed: Ptr<CPlayer>, style: KillStyle) {
        unsafe {
            killer.killsinrow += 1;

            if killer.inair
                && (style == KillStyle::Stomp
                    || style == KillStyle::Goomba
                    || style == KillStyle::Koopa
                    || style == KillStyle::CheepCheep
                    || style == KillStyle::BulletBill
                    || style == KillStyle::Feather)
            {
                killer.killsinrowinair += 1;
            }

            //Play announcer
            let mut fSoundPlayed = false;
            if killer.killsinrowinair > 1 {
                if rm.sfx_announcer[9].play() {
                    fSoundPlayed = true;
                }
            }

            if !killed.is_null() && killed.killsinrow >= 2 && !fSoundPlayed {
                if rm.sfx_announcer[10].play() {
                    fSoundPlayed = true;
                }
            }

            let mut awardIndex: i16 = 0;
            if killer.killsinrow >= 2 {
                awardIndex = if (killer.killsinrow as i32 - 2) >= PAWARD_LAST as i32 {
                    PAWARD_LAST - 1
                } else {
                    (killer.killsinrow as i32 - 2) as i16
                };

                if !fSoundPlayed {
                    if rm.sfx_announcer[awardIndex as usize].play() {
                        fSoundPlayed = true;
                    }
                }

                if killer.killsinrow >= 5 {
                    game_values.unlocksecret2part1 = true;
                    check_secret(1);
                }
            }

            //Add eyecandy
            if game_values.awardstyle != AwardStyle::None {
                let slot = ((killer.killsinrow as i32 - 1) % MAXAWARDS) as usize;
                if game_values.awardstyle == AwardStyle::Halo {
                    killer.awardeffects.awards[slot] = style as i16;
                } else if game_values.awardstyle == AwardStyle::Souls {
                    if !killed.is_null() {
                        killer.awardeffects.awards[slot] = killed.get_color_id();
                    } else if style == KillStyle::Goomba {
                        killer.awardeffects.awards[slot] = 4; //soul id for goomba
                    } else if style == KillStyle::BulletBill {
                        killer.awardeffects.awards[slot] = 5; //soul id for bullet bill
                    } else if style == KillStyle::CheepCheep {
                        killer.awardeffects.awards[slot] = 6; //soul id for cheep cheep
                    } else if style == KillStyle::Koopa {
                        killer.awardeffects.awards[slot] = 7; //soul id for koopa
                    } else {
                        killer.awardeffects.awards[slot] = 8; //soul id for ?
                    }
                } else if game_values.awardstyle == AwardStyle::Swirl {
                    Ptr::from_mut(&mut killer.awardeffects).get().add_swirling(killer);
                } else if game_values.awardstyle == AwardStyle::Fireworks {
                    Ptr::from_mut(&mut killer.awardeffects).get().add_rocket(killer);
                }

                if killer.killsinrowinair > 1 {
                    Ptr::from_mut(&mut killer.awardeffects).get().add_kills_in_row_in_air_award(killer);
                }

                //if we have enough kills in a row -> spawn an award

                if game_values.awardstyle == AwardStyle::Text {
                    if killer.killsinrow >= 2 {
                        let text = format!("{} - {}", killer.killsinrow, awardtexts[awardIndex as usize].name);

                        let font = if awardtexts[awardIndex as usize].large {
                            Ptr::from_mut(&mut rm.game_font_large)
                        } else {
                            Ptr::from_mut(&mut rm.game_font_small)
                        };
                        eyecandy[2].emplace(EC_GravText::new(font, killer.center_x(), killer.bottom_y(), text, -VELJUMP));
                    }

                    //if we stopped the other players run show another award
                    if !killed.is_null() && killed.killsinrow >= 2 {
                        let a: i16 = if (killed.killsinrow as i32 - 2) >= PAWARD_LAST as i32 {
                            PAWARD_LAST - 1
                        } else {
                            (killed.killsinrow as i32 - 2) as i16
                        };
                        let text = format!("{} Stopped!", awardtexts[a as usize].name);

                        let font = if awardtexts[a as usize].large {
                            Ptr::from_mut(&mut rm.game_font_large)
                        } else {
                            Ptr::from_mut(&mut rm.game_font_small)
                        };
                        eyecandy[2].emplace(EC_GravText::new(font, killed.center_x(), killed.bottom_y(), text, -VELJUMP * 1.3f32));
                    }
                }
            }
        }
    }

    pub fn add_kills_in_row_in_air_award(&mut self, player: &mut CPlayer) {
        unsafe {
            let mut angle: f32 = 0.0f32;
            for k in 0..15i16 {
                let vel: f32 = 7.0f32 + ((k % 2) as f32 * 5.0f32);
                let awardvelx: f32 = vel * cosf(angle);
                let awardvely: f32 = vel * sinf(angle);

                eyecandy[2].emplace(EC_FallingObject::new(
                    Ptr::from_mut(&mut rm.spr_bonus),
                    (player.center_x() as i32 - 8) as i16,
                    (player.center_y() as i32 - 8) as i16,
                    awardvelx,
                    awardvely,
                    4,
                    2,
                    0,
                    (player.get_color_id() as i32 * 16) as i16,
                    16,
                    16,
                ));
                angle -= PI / 14.0f32;
            }

            //Track to unlock secret
            game_values.unlocksecret1part1[player.get_global_id() as usize] = true;
            check_secret(0);
        }
    }
}
