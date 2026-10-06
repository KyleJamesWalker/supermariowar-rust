//! Port of src/smw/gamemodes/Star.cpp

use crate::common::eyecandy::{EC_GravText, EC_SingleAnimation};
use crate::common::game_mode::*;
use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::StarStyle;
use crate::common::global_constants::VELJUMP;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::*;
use crate::smw::gamemodes::time_limit::{cgm_timelimit_init, CGM_TimeLimit};
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::{players, score, score_cnt};
use crate::smw::net_random::{self, Ev};
use crate::smw::objects::carriable::co_star::CO_Star;
use crate::smw::objects::moving::mo_carried_object::MO_CarriedObjectTrait;
use crate::smw::player::CPlayer;

//Star mode - shared timer ticks down and the players must pass around
//a hot potato.  When the timer hits zero, the player with the star
//loses a point.
pub struct CGM_Star {
    pub cgm_time_limit: CGM_TimeLimit,

    pub starItem: [Ptr<CO_Star>; 3],
    pub starPlayer: [Ptr<CPlayer>; 3],
    pub iCurrentModeType: StarStyle,
    pub fDisplayTimer: bool,
}
impl_base!(CGM_Star => cgm_time_limit: CGM_TimeLimit);

impl CGM_Star {
    pub fn new() -> Self {
        let mut this = CGM_Star {
            cgm_time_limit: CGM_TimeLimit::new(),
            starItem: [Ptr::null(); 3],
            starPlayer: [Ptr::null(); 3],
            iCurrentModeType: StarStyle::Ztar,
            fDisplayTimer: false,
        };
        this.goal = 5;
        this.gamemode = game_mode_star;
        this.setup_mode_strings("Star", "Lives", 1);
        this.iCurrentModeType = StarStyle::Ztar;
        this
    }

    pub fn getstarplayer(&mut self, id: i16) -> Ptr<CPlayer> {
        self.starPlayer[id as usize]
    }

    pub fn getcurrentmodetype(&self) -> StarStyle {
        self.iCurrentModeType
    }

    fn setup_mode(&mut self) {
        unsafe {
            //Clean up old stars
            for iStar in 0..3usize {
                if !self.starItem[iStar].is_null() {
                    MO_CarriedObjectTrait::drop(&mut *self.starItem[iStar]);
                    self.starItem[iStar].dead = true;
                    self.starItem[iStar] = Ptr::null();
                }

                self.starPlayer[iStar] = Ptr::null();
            }

            //If multi star, add more stars
            if self.iCurrentModeType == StarStyle::Multi {
                let mut rankedPlayers: [Ptr<CPlayer>; 4] = [Ptr::null(); 4];
                let fReverseScoring = self.fReverseScoring;
                let iNumPlayers: i16 = self.get_score_ranked_player_list(&mut rankedPlayers, fReverseScoring);

                let mut iStar: i16 = 0;
                while (iStar as i32) < iNumPlayers as i32 - 1 {
                    self.starPlayer[iStar as usize] = rankedPlayers[iStar as usize];

                    self.starItem[iStar as usize] = Ptr::new_box(CO_Star::new(Ptr::from_mut(&mut rm.spr_star), 1, iStar));
                    self.starItem[iStar as usize].set_player_color(self.starPlayer[iStar as usize].get_color_id());
                    objectcontainer[1].add(self.starItem[iStar as usize]);
                    iStar += 1;
                }
            } else {
                //otherwise, add just a single star
                let fGetHighest = !self.fReverseScoring && self.iCurrentModeType == StarStyle::Ztar;
                self.starPlayer[0] = self.get_highest_score_player(fGetHighest);

                self.starItem[0] = Ptr::new_box(CO_Star::new(Ptr::from_mut(&mut rm.spr_star), if self.iCurrentModeType == StarStyle::Ztar { 0 } else { 1 }, 0));
                objectcontainer[1].add(self.starItem[0]);
            }
        }
    }

    pub fn isplayerstar(&mut self, player: Ptr<CPlayer>) -> bool {
        unsafe {
            let mut iPlayer: i16 = 0;
            while (iPlayer as usize) < players.len().wrapping_sub(1) {
                if self.starPlayer[iPlayer as usize] == player {
                    return true;
                }
                iPlayer += 1;
            }
        }

        false
    }

    pub fn swapplayer(&mut self, id: i16, player: Ptr<CPlayer>) -> Ptr<CPlayer> {
        unsafe {
            let mut oldstar: Ptr<CPlayer> = Ptr::null();
            if !self.starPlayer[id as usize].is_null() {
                oldstar = self.starPlayer[id as usize];
                oldstar.shield().turn_on();
                eyecandy[2].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut rm.spr_fireballexplosion),
                    (oldstar.center_x() as i32 - 16) as i16,
                    (oldstar.center_y() as i32 - 16) as i16,
                    3,
                    8,
                ));
            }

            self.starPlayer[id as usize] = player;

            if self.iCurrentModeType == StarStyle::Multi {
                let iColor = self.starPlayer[id as usize].get_color_id();
                self.starItem[id as usize].set_player_color(iColor);
            }

            if self.starItem[id as usize].get_type() == 1 {
                let text = if self.iCurrentModeType == StarStyle::Multi { "Star Get!" } else { "Shine Get!" };
                eyecandy[2].emplace(EC_GravText::new(
                    Ptr::from_mut(&mut rm.game_font_large),
                    player.center_x(),
                    player.bottom_y(),
                    text.to_string(),
                    (-(VELJUMP as f64) * 1.5) as f32,
                ));
            } else {
                eyecandy[2].emplace(EC_GravText::new(
                    Ptr::from_mut(&mut rm.game_font_large),
                    player.center_x(),
                    player.bottom_y(),
                    "Ztarred!".to_string(),
                    (-(VELJUMP as f64) * 1.5) as f32,
                ));
            }

            eyecandy[2].emplace(EC_SingleAnimation::new(
                Ptr::from_mut(&mut rm.spr_fireballexplosion),
                (player.center_x() as i32 - 16) as i16,
                (player.center_y() as i32 - 16) as i16,
                3,
                8,
            ));
            if_sound_on_play(&mut rm.sfx_transform);

            oldstar
        }
    }

    fn rebalance_multi_stars(&mut self) {
        let mut rankedPlayers: [Ptr<CPlayer>; 4] = [Ptr::null(); 4];
        let fReverseScoring = self.fReverseScoring;
        let iNumPlayers: i16 = self.get_score_ranked_player_list(&mut rankedPlayers, fReverseScoring);

        let mut iStar: i16 = 0;
        while (iStar as i32) < iNumPlayers as i32 - 1 {
            self.starPlayer[iStar as usize] = rankedPlayers[iStar as usize];
            let iColor = self.starPlayer[iStar as usize].get_color_id();
            self.starItem[iStar as usize].set_player_color(iColor);
            self.starItem[iStar as usize].place_star();
            iStar += 1;
        }
    }
}

impl CGameModeTrait for CGM_Star {
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self) {
        cgm_star_init(self)
    }
    fn think(&mut self) {
        cgm_star_think(self)
    }
    fn draw_foreground(&mut self) {
        if self.fDisplayTimer {
            self.gameClock.draw();
        }
    }
    fn playerkilledplayer(&mut self, _inflictor: Ptr<CPlayer>, _other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        PlayerKillType::Normal
    }
    fn playerkilledself(&mut self, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgamemode_playerkilledself(self, player, style)
    }
    fn playerextraguy(&mut self, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            if iType == 5 {
                player.score().adjust_score(if self.fReverseScoring { -1 } else { 1 });
            } else {
                self.gameClock.add_time((iType as i32 * 10) as i16);
            }
        }
    }
}

fn star_time() -> i16 {
    unsafe {
        if game_values.gamemodesettings.star.time < 1 {
            30
        } else {
            game_values.gamemodesettings.star.time
        }
    }
}

pub fn cgm_star_init(this: &mut CGM_Star) {
    cgm_timelimit_init(&mut this.cgm_time_limit);

    this.fDisplayTimer = true;

    this.gameClock.init(star_time(), true);

    unsafe {
        this.iCurrentModeType = game_values.gamemodesettings.star.shine;
        if this.iCurrentModeType == StarStyle::Random {
            this.iCurrentModeType = StarStyle::from_u8(RANDOM_INT(3) as u8);
        }

        this.fReverseScoring = this.goal == -1;

        //Set initial scores
        for iScore in 0..score_cnt {
            if this.fReverseScoring {
                score[iScore as usize].set_score(0);
            } else {
                score[iScore as usize].set_score(this.goal);
            }
        }
    }

    for iStar in 0..3usize {
        this.starItem[iStar] = Ptr::null();
        this.starPlayer[iStar] = Ptr::null();
    }

    this.setup_mode();
}

fn star_needs_reassign(this: &CGM_Star) -> bool {
    unsafe {
        if this.iCurrentModeType == StarStyle::Multi {
            (0..players.len().saturating_sub(1)).any(|i| this.starPlayer[i].is_null())
        } else {
            this.starPlayer[0].is_null()
        }
    }
}

fn reassign(this: &mut CGM_Star) {
    unsafe {
        if this.iCurrentModeType == StarStyle::Multi {
            let mut iStar1: usize = 0;
            while iStar1 + 1 < players.len() {
                //If we're missing a star player, then reassign them all
                if this.starPlayer[iStar1].is_null() {
                    this.rebalance_multi_stars();
                    break;
                }
                iStar1 += 1;
            }
        } else if this.starPlayer[0].is_null() {
            let fGetHighest = !this.fReverseScoring && game_values.gamemodesettings.star.shine == StarStyle::Ztar;
            this.starPlayer[0] = this.get_highest_score_player(fGetHighest);
            this.starItem[0].place_star();
        }
    }
}

fn time_out(this: &mut CGM_Star) {
    unsafe {
        this.gameClock.set_time(star_time());
        if_sound_on_play(&mut rm.sfx_thunder);

        if this.iCurrentModeType == StarStyle::Ztar {
            if score[this.starPlayer[0].get_team_id() as usize].score > 1 || this.fReverseScoring {
                this.starPlayer[0].kill_player_map_hazard(true, KillStyle::Environment, false, -1);
            }

            if this.fReverseScoring {
                this.starPlayer[0].score().adjust_score(1);
            } else {
                this.starPlayer[0].score().adjust_score(-1);

                if this.starPlayer[0].score().score <= 0 {
                    this.fDisplayTimer = !remove_team(this.starPlayer[0].get_team_id());
                    this.starPlayer[0] = Ptr::null();
                }
            }

            let fGetHighest = !this.fReverseScoring;
            this.starPlayer[0] = this.get_highest_score_player(fGetHighest);
            this.starItem[0].place_star();
        } else if this.iCurrentModeType == StarStyle::Shine {
            for i in 0..players.len() {
                let mut player = players[i];
                if this.starPlayer[0].get_team_id() == player.get_team_id() {
                    continue;
                }

                //Let the cleanup function remove the player on the last kill
                if score[player.get_team_id() as usize].score > 1 || this.fReverseScoring {
                    player.kill_player_map_hazard(true, KillStyle::Environment, false, -1);
                }
            }

            if this.fReverseScoring {
                this.starPlayer[0].score().adjust_score(1);
            } else {
                for iTeam in 0..score_cnt {
                    if this.starPlayer[0].get_team_id() == iTeam {
                        continue;
                    }

                    score[iTeam as usize].adjust_score(-1);

                    if score[iTeam as usize].score <= 0 {
                        this.fDisplayTimer = !remove_team(iTeam);
                    }
                }
            }

            this.starPlayer[0] = this.get_highest_score_player(false);
            this.starItem[0].place_star();
        } else if this.iCurrentModeType == StarStyle::Multi {
            for iPlayer in 0..players.len() {
                let mut fFound = false;
                let mut iStar: usize = 0;
                while iStar + 1 < players.len() {
                    if this.starPlayer[iStar] == players[iPlayer] {
                        fFound = true;
                        break;
                    }
                    iStar += 1;
                }

                if fFound {
                    continue;
                }

                let mut p = players[iPlayer];
                if score[p.get_team_id() as usize].score > 1 || this.fReverseScoring {
                    p.kill_player_map_hazard(true, KillStyle::Environment, false, -1);
                }

                let mut fNeedRebalance = true;
                let mut p = players[iPlayer];
                if this.fReverseScoring {
                    p.score().adjust_score(1);
                } else {
                    p.score().adjust_score(-1);

                    if p.score().score <= 0 {
                        this.fDisplayTimer = !remove_team(p.get_team_id());

                        if game_values.gamemodesettings.star.shine != StarStyle::Random {
                            this.setup_mode();
                            fNeedRebalance = false;
                        }
                    }
                }

                if game_values.gamemodesettings.star.shine != StarStyle::Random && fNeedRebalance {
                    this.rebalance_multi_stars();
                }

                break;
            }
        }

        //Play warning sound if needed
        if !this.fReverseScoring && !this.playedwarningsound {
            let mut countscore: i16 = 0;
            for j in 0..score_cnt {
                for k in 0..score_cnt {
                    if j == k {
                        continue;
                    }

                    countscore = (countscore as i32 + score[k as usize].score as i32) as i16;
                }

                if countscore <= 1 {
                    this.playwarningsound();
                    break;
                }

                countscore = 0;
            }
        }

        //If random game, then choose a new game type
        if game_values.gamemodesettings.star.shine == StarStyle::Random && this.fDisplayTimer {
            this.iCurrentModeType = StarStyle::from_u8(RANDOM_INT(3) as u8);
            this.setup_mode();
        }
    }
}

fn star_mode() -> Option<&'static mut CGM_Star> {
    unsafe { game_values.gamemode.as_any().downcast_mut::<CGM_Star>() }
}

pub fn net_reassign() {
    if let Some(this) = star_mode() {
        if star_needs_reassign(this) {
            reassign(this);
        }
    }
}

pub fn net_timeout() {
    if let Some(this) = star_mode() {
        if !star_needs_reassign(this) {
            time_out(this);
        }
    }
}

pub fn net_state() -> String {
    match star_mode() {
        Some(this) => {
            let ids: Vec<String> = this.starPlayer.iter().map(|p| if p.is_null() { "-".to_string() } else { p.globalID.to_string() }).collect();
            format!("mode{} stars{}", this.iCurrentModeType as i32, ids.join(","))
        }
        None => String::new(),
    }
}

/// Replay-harness `T` record (docs/REPLAY.md).
pub fn harness_record() -> Option<String> {
    star_mode().map(|this| {
        let ids: Vec<String> = this.starPlayer.iter().map(|p| if p.is_null() { "-1".to_string() } else { p.globalID.to_string() }).collect();
        format!("T type={} holders={}", this.iCurrentModeType as i32, ids.join(","))
    })
}

pub fn cgm_star_think(this: &mut CGM_Star) {
    if this.gameover {
        this.displayplayertext();
        return;
    }

    unsafe {
        //Make sure there is a star player(s)
        if star_needs_reassign(this) && !net_random::event(Ev::StarReassign, &[]) {
            reassign(this);
        }

        //Count down the game time
        let iTime: i16 = this.gameClock.run_clock();
        if iTime <= 5 && iTime > 0 {
            if_sound_on_play(&mut rm.sfx_starwarning);
        }

        //If the game time ran out, somebody needs to die and scores changed
        if iTime == 0 && !net_random::event(Ev::StarTimeout, &[]) {
            time_out(this);
        }
    }
}
