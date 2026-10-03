//! Port of src/smw/gamemodes/Greed.cpp

use crate::common::game_mode::*;
use crate::common::game_values::if_sound_on_play;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::classic::CGM_Classic;
use crate::smw::gamemodes::game_mode::*;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::{score, score_cnt};
use crate::smw::objects::moving::mo_coin::MO_Coin;
use crate::smw::player::CPlayer;

const fn kill_style_damage(style: KillStyle) -> i16 {
    match style {
        KillStyle::Stomp => 5,
        KillStyle::Star => 5,
        KillStyle::Fireball => 3,
        KillStyle::Bobomb => 8,
        KillStyle::Bounce => 3,
        KillStyle::Pow => 5,
        KillStyle::Goomba => 2,
        KillStyle::BulletBill => 3,
        KillStyle::Hammer => 3,
        KillStyle::Shell => 5,
        KillStyle::ThrowBlock => 5,
        KillStyle::CheepCheep => 2,
        KillStyle::Koopa => 2,
        KillStyle::Boomerang => 3,
        KillStyle::Feather => 5,
        KillStyle::IceBlast => 8,
        KillStyle::Podobo => 3,
        KillStyle::Bomb => 8,
        KillStyle::Leaf => 5,
        KillStyle::PWings => 5,
        KillStyle::KuriboShoe => 8,
        KillStyle::PoisonMushroom => 5,
        KillStyle::Environment => 3,
        KillStyle::Push => 3,
        KillStyle::BuzzyBeetle => 2,
        KillStyle::Spiny => 2,
        KillStyle::Phanto => 2,
    }
}

//Greed mode (players try to steal each other's coins)
pub struct CGM_Greed {
    pub cgm_classic: CGM_Classic,
}
impl_base!(CGM_Greed => cgm_classic: CGM_Classic);

//Greed - steal other players coins - if you have 0 coins, you're removed from the game!
impl CGM_Greed {
    pub fn new() -> Self {
        let mut this = CGM_Greed { cgm_classic: CGM_Classic::new() };
        this.goal = 40;
        this.gamemode = game_mode_greed;

        this.setup_mode_strings("Greed", "Coins", 10);
        this
    }

    pub fn release_coins(&mut self, mut player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        unsafe {
            if_sound_on_play(&mut rm.sfx_cannon);

            player.shield().turn_on();

            let mut iDamage: i16 = (kill_style_damage(style) as i32 * game_values.gamemodesettings.greed.multiplier as i32 / 2) as i16;

            if self.goal != -1 {
                if player.score().score < iDamage {
                    iDamage = player.score().score;
                }

                player.score().adjust_score(-iDamage);
            }

            let pos: Vec2s = Vec2s::new((player.center_x() as i32 - 16) as i16, (player.center_y() as i32 - 16) as i16);

            for _k in 0..iDamage {
                let speed: f32 = 7.0f32 + (RANDOM_INT(9) as f32) / 2.0f32;
                let angle: f32 = -(RANDOM_INT(314) as f32) / 100.0f32;
                let vel: Vec2f = Vec2f::new(speed * angle.cos(), speed * angle.sin());

                objectcontainer[1].add(Ptr::new_box(MO_Coin::new(Ptr::from_mut(&mut rm.spr_coin), vel, pos, player.get_color_id(), player.get_team_id(), 1, 30, false)));
            }

            //Play warning sound if game is almost over
            if self.goal != -1 && !self.playedwarningsound {
                let mut playwarning = false;
                for j in 0..score_cnt {
                    let mut countscore: i16 = 0;
                    for k in 0..score_cnt {
                        if j == k {
                            continue;
                        }

                        countscore = (countscore as i32 + score[k as usize].score as i32) as i16;
                    }

                    if countscore <= 10 {
                        playwarning = true;
                        break;
                    }
                }

                if playwarning {
                    self.playwarningsound();
                }
            }

            if self.goal != -1 && player.score().score <= 0 {
                remove_team(player.get_team_id());
                return PlayerKillType::Removed;
            }

            PlayerKillType::NonKill
        }
    }
}

impl CGameModeTrait for CGM_Greed {
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self) {
        cgamemode_init(self);

        let iGoal: i16 = if self.goal == -1 { 0 } else { self.goal };

        unsafe {
            for iScore in 0..score_cnt {
                score[iScore as usize].set_score(iGoal);
            }
        }
    }

    fn playerkilledplayer(&mut self, _inflictor: Ptr<CPlayer>, other: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        if self.gameover {
            return PlayerKillType::Normal;
        }

        //create coins around player
        self.release_coins(other, style)
    }

    fn playerkilledself(&mut self, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        if self.gameover {
            return PlayerKillType::Normal;
        }

        //create coins around player
        self.release_coins(player, style)
    }

    fn playerextraguy(&mut self, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            player.score().adjust_score((iType as i32 * 5) as i16);
        }
    }
}
