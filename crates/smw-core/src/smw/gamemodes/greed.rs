//! Greed mode: getting hit spills coins; a team with no coins left is removed.

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::game_mode::*;
use crate::common::game_values::if_sound_on_play;
use crate::common::math::trig::{cosf, sinf};
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::global_constants::{ticks, vel};
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::classic::CGM_Classic;
use crate::smw::gamemodes::game_mode::*;

use crate::smw::objects::moving::mo_coin::MO_Coin;
use crate::smw::player::CPlayer;

/// Base coins lost to each kill style, before the greed multiplier (in halves) is applied.
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

/// Greed mode: teams start with the goal in coins and steal each other's by knocking them loose.
#[derive(Clone)]
pub struct CGM_Greed {
    pub cgm_classic: CGM_Classic,
}
impl_base!(CGM_Greed => cgm_classic: CGM_Classic);

/// Spawns `iDamage` coins at `pos` that fly out in random upward directions.
fn drop_coins(g: &mut Game, player: PlayerId, iDamage: i16, pos: Vec2s) {
        let mut gp = g.ptr();
    for _k in 0..iDamage {
        let speed: f32 = vel(7.0f32 + (RANDOM_INT(gp.get(), 9) as f32) / 2.0f32);
        let angle: f32 = -(RANDOM_INT(gp.get(), 314) as f32) / 100.0f32;
        let vel: Vec2f = Vec2f::new(speed * cosf(angle), speed * sinf(angle));

        let obj = Ptr::new_box(MO_Coin::new(g, gp.get().rm.spr_coin, vel, pos, g.player(player).get_color_id(), g.player(player).get_team_id(), 1, ticks(30) as i16, false));
        gp.get().objectcontainer[1].add(g, obj);
    }
}

impl CGM_Greed {
    pub fn new() -> Self {
        let mut this = CGM_Greed { cgm_classic: CGM_Classic::new() };
        this.goal = 40;
        this.gamemode = game_mode_greed;

        this.setup_mode_strings("Greed", "Coins", 10);
        this
    }

    /// Spills the player's coins for a hit by `style` and shields them instead of killing them.
    /// Returns `Removed` when the team runs out of coins, otherwise `NonKill`.
    pub fn release_coins(&mut self, g: &mut Game, mut player: PlayerId, style: KillStyle) -> PlayerKillType {
        let mut gp = g.ptr();
        if_sound_on_play(g, &mut gp.rm.sfx_cannon);

        g.player(player).shield().turn_on(g);

        let mut iDamage: i16 = (kill_style_damage(style) as i32 * g.game_values.gamemodesettings.greed.multiplier as i32 / 2) as i16;

        if self.goal != -1 {
            if g.player(player).score(g).score < iDamage {
                iDamage = g.player(player).score(g).score;
            }

            g.player(player).score(g).adjust_score(g, -iDamage);
        }

        let pos: Vec2s = Vec2s::new((g.player(player).center_x() as i32 - 16) as i16, (g.player(player).center_y() as i32 - 16) as i16);

        drop_coins(g, player, iDamage, pos);

        //Play warning sound if game is almost over
        if self.goal != -1 && !self.playedwarningsound {
            let mut playwarning = false;
            for j in 0..g.score_cnt {
                let mut countscore: i16 = 0;
                for k in 0..g.score_cnt {
                    if j == k {
                        continue;
                    }

                    countscore = (countscore as i32 + g.score[k as usize].score as i32) as i16;
                }

                if countscore <= 10 {
                    playwarning = true;
                    break;
                }
            }

            if playwarning {
                self.playwarningsound(g);
            }
        }

        if self.goal != -1 && g.player(player).score(g).score <= 0 {
            remove_team(g, g.player(player).get_team_id());
            return PlayerKillType::Removed;
        }

        PlayerKillType::NonKill
    }
}

impl CGameModeTrait for CGM_Greed {
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        cgamemode_init(g, self);

        let iGoal: i16 = if self.goal == -1 { 0 } else { self.goal };

        for iScore in 0..g.score_cnt {
            gp.score[iScore as usize].set_score(g, iGoal);
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, _inflictor: PlayerId, other: PlayerId, style: KillStyle) -> PlayerKillType {
        if self.gameover {
            return PlayerKillType::Normal;
        }

        //create coins around player
        self.release_coins(g, other, style)
    }

    fn playerkilledself(&mut self, g: &mut Game, player: PlayerId, style: KillStyle) -> PlayerKillType {
        if self.gameover {
            return PlayerKillType::Normal;
        }

        //create coins around player
        self.release_coins(g, player, style)
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: PlayerId, iType: i16) {
        if !self.gameover {
            g.player(player).score(g).adjust_score(g, (iType as i32 * 5) as i16);
        }
    }
}
