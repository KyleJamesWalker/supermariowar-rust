//! Tag mode: the tagged player bleeds points until they tag someone else.

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::eyecandy::{EC_GravText, EC_SingleAnimation};
use crate::common::game_mode::*;
use crate::common::game_values::if_sound_on_play;
use crate::common::global_constants::{ticks, VELJUMP};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::*;

use crate::smw::player::CPlayer;

#[derive(Clone)]

/// Tag mode. Teams start at the goal; the tagged player loses a point every `ticks(pointspeed)`
/// ticks and passes the tag by killing someone. Any death costs 5; a team is removed at zero.
pub struct CGM_Tag {
    pub cgame_mode: CGameMode,

    /// The tagged player, or `PlayerId::NONE` until `think` reassigns it.
    pub m_tagged: PlayerId,
    /// Ticks toward the tagged player's next point loss; `init` does not reset it.
    counter: i16,
}
impl_base!(CGM_Tag => cgame_mode: CGameMode);

/// Returns true when, for some team, the other teams' scores total at most 20% of the goal.
fn tag_warning_needed(g: &mut Game, goal: i16) -> bool {
    let mut countscore: i16 = 0;
    let mut playwarning = false;
    for j in 0..g.score_cnt {
        for k in 0..g.score_cnt {
            if j == k {
                continue;
            }

            countscore = (countscore as i32 + g.score[k as usize].score as i32) as i16;
        }

        if countscore as f64 <= goal as f64 * 0.2 {
            playwarning = true;
            break;
        }

        countscore = 0;
    }
    playwarning
}

impl CGM_Tag {
    /// Tags the highest-scoring living player (lowest under reverse scoring), breaking ties randomly.
    fn reassign(&mut self, g: &mut Game) {
        let fGetHighest = !self.fReverseScoring;
        self.m_tagged = self.get_highest_score_player(g, fGetHighest);
    }

    pub fn new() -> Self {
        let mut this = CGM_Tag { cgame_mode: CGameMode::new(), m_tagged: PlayerId::NONE, counter: 0 };
        this.goal = 200;
        this.gamemode = game_mode_tag;

        this.setup_mode_strings("Tag", "Points", 50);
        this
    }

    pub fn tagged(&self) -> PlayerId {
        self.m_tagged
    }

    pub fn set_tagged(&mut self, player: PlayerId) {
        self.m_tagged = player;
    }
}

impl CGameModeTrait for CGM_Tag {
    fn harness_hash(&self, h: &mut crate::smw::rollback::Fnv) {
        self.gm().harness_hash(h);
        h.player(self.m_tagged);
        h.i16s(&[self.counter]);
    }
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        cgamemode_init(g, self);
        self.fReverseScoring = self.goal == -1;

        for iScore in 0..g.score_cnt {
            if self.fReverseScoring {
                gp.score[iScore as usize].set_score(g, 0);
            } else {
                gp.score[iScore as usize].set_score(g, self.goal);
            }
        }

        self.set_tagged(PlayerId::NONE);
    }

    fn think(&mut self, g: &mut Game) {
        if self.gameover {
            self.displayplayertext(g);
        } else {
            if self.m_tagged.is_none() {
                self.reassign(g);
            }
            if self.m_tagged.is_none() {
                return;
            }

            let mut tagged = self.m_tagged;
            if g.player(tagged).isready() {
                self.counter += 1;
                if self.counter as i32 >= ticks(g.game_values.pointspeed as i32) {
                    self.counter = 0;

                    if self.fReverseScoring {
                        g.player(tagged).score(g).adjust_score(g, 1);
                    } else {
                        g.player(tagged).score(g).adjust_score(g, -1);
                    }
                }
            }

            if self.fReverseScoring {
                return;
            }

            let playwarning = tag_warning_needed(g, self.goal);

            if playwarning && !self.playedwarningsound {
                self.playwarningsound(g);
            }

            let mut tagged = self.m_tagged;
            if g.player(tagged).score(g).score <= 0 {
                remove_team(g, g.player(tagged).get_team_id());
                self.m_tagged = PlayerId::NONE;
            }
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, mut inflictor: PlayerId, mut other: PlayerId, _style: KillStyle) -> PlayerKillType {
        let mut gp = g.ptr();
        if inflictor == self.m_tagged {
            self.m_tagged = other;
            g.player(inflictor).shield().reset(g);
            g.eyecandy[2].emplace(EC_GravText::new(
                Ptr::from_mut(&mut g.rm.game_font_large),
                g.player(other).center_x(),
                g.player(other).bottom_y(),
                "Tagged!".to_string(),
                (-(VELJUMP as f64) * 1.5) as f32,
            ));
            g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, 
                g.rm.spr_fireballexplosion,
                (g.player(other).center_x() as i32 - 16) as i16,
                (g.player(other).center_y() as i32 - 16) as i16,
                3,
                8,
            ));
            if_sound_on_play(g, &mut gp.rm.sfx_transform);
        }

        if !self.gameover {
            if self.fReverseScoring {
                g.player(other).score(g).adjust_score(g, 5);
                return PlayerKillType::Normal;
            } else {
                g.player(other).score(g).adjust_score(g, -5);
            }

            let mut countscore: i16 = 0;
            for k in 0..g.score_cnt {
                if g.player(inflictor).score_slot == (k as usize) as usize {
                    continue;
                }

                countscore = (countscore as i32 + g.score[k as usize].score as i32) as i16;
            }

            if countscore as f64 <= self.goal as f64 * 0.2 && !self.playedwarningsound {
                self.playwarningsound(g);
            }

            if g.player(other).score(g).score <= 0 {
                g.player(other).score(g).set_score(g, 0);

                remove_team(g, g.player(other).get_team_id());
                return PlayerKillType::Removed;
            }
        }

        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, g: &mut Game, mut player: PlayerId, style: KillStyle) -> PlayerKillType {
        cgamemode_playerkilledself(g, self, player, style);

        if !self.gameover {
            if self.fReverseScoring {
                g.player(player).score(g).adjust_score(g, 5);
                return PlayerKillType::Normal;
            } else {
                g.player(player).score(g).adjust_score(g, -5);
            }

            let playwarning = tag_warning_needed(g, self.goal);

            if playwarning && !self.playedwarningsound {
                self.playwarningsound(g);
            }

            if g.player(player).score(g).score <= 0 {
                g.player(player).score(g).set_score(g, 0);
                remove_team(g, g.player(player).get_team_id());
                return PlayerKillType::Removed;
            }
        }

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: PlayerId, iType: i16) {
        if !self.gameover {
            if self.fReverseScoring {
                g.player(player).score(g).adjust_score(g, (-10 * iType as i32) as i16);
            } else {
                g.player(player).score(g).adjust_score(g, (10 * iType as i32) as i16);
            }
        }
    }
}

fn tag_mode(g: &mut Game) -> Option<&'static mut CGM_Tag> {
    let mut gp = g.ptr();
    gp.mode().get().as_any().downcast_mut::<CGM_Tag>()
}

