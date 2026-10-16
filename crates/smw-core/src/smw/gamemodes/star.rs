//! Port of src/smw/gamemodes/Star.cpp

use crate::game::Game;
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

    fn setup_mode(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        unsafe {
            //Clean up old stars
            for iStar in 0..3usize {
                if !self.starItem[iStar].is_null() {
                    MO_CarriedObjectTrait::drop(&mut *self.starItem[iStar], g);
                    self.starItem[iStar].dead = true;
                    self.starItem[iStar] = Ptr::null();
                }

                self.starPlayer[iStar] = Ptr::null();
            }

            //If multi star, add more stars
            if self.iCurrentModeType == StarStyle::Multi {
                let mut rankedPlayers: [Ptr<CPlayer>; 4] = [Ptr::null(); 4];
                let fReverseScoring = self.fReverseScoring;
                let iNumPlayers: i16 = self.get_score_ranked_player_list(g, &mut rankedPlayers, fReverseScoring);

                let mut iStar: i16 = 0;
                while (iStar as i32) < iNumPlayers as i32 - 1 {
                    self.starPlayer[iStar as usize] = rankedPlayers[iStar as usize];

                    self.starItem[iStar as usize] = Ptr::new_box(CO_Star::new(g, Ptr::from_mut(&mut gp.rm.spr_star), 1, iStar));
                    self.starItem[iStar as usize].set_player_color(self.starPlayer[iStar as usize].get_color_id());
                    gp.objectcontainer[1].add(g, self.starItem[iStar as usize]);
                    iStar += 1;
                }
            } else {
                //otherwise, add just a single star
                let fGetHighest = !self.fReverseScoring && self.iCurrentModeType == StarStyle::Ztar;
                self.starPlayer[0] = self.get_highest_score_player(g, fGetHighest);

                self.starItem[0] = Ptr::new_box(CO_Star::new(g, Ptr::from_mut(&mut gp.rm.spr_star), if self.iCurrentModeType == StarStyle::Ztar { 0 } else { 1 }, 0));
                gp.objectcontainer[1].add(g, self.starItem[0]);
            }
        }
    }

    pub fn isplayerstar(&mut self, g: &mut Game, player: Ptr<CPlayer>) -> bool {
        unsafe {
            let mut iPlayer: i16 = 0;
            while (iPlayer as usize) < g.players.len().wrapping_sub(1) {
                if self.starPlayer[iPlayer as usize] == player {
                    return true;
                }
                iPlayer += 1;
            }
        }

        false
    }

    pub fn swapplayer(&mut self, g: &mut Game, id: i16, player: Ptr<CPlayer>) -> Ptr<CPlayer> {
        let mut gp = g.ptr();
        unsafe {
            let mut oldstar: Ptr<CPlayer> = Ptr::null();
            if !self.starPlayer[id as usize].is_null() {
                oldstar = self.starPlayer[id as usize];
                oldstar.shield().turn_on(g);
                g.eyecandy[2].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut g.rm.spr_fireballexplosion),
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
                g.eyecandy[2].emplace(EC_GravText::new(
                    Ptr::from_mut(&mut g.rm.game_font_large),
                    player.center_x(),
                    player.bottom_y(),
                    text.to_string(),
                    (-(VELJUMP as f64) * 1.5) as f32,
                ));
            } else {
                g.eyecandy[2].emplace(EC_GravText::new(
                    Ptr::from_mut(&mut g.rm.game_font_large),
                    player.center_x(),
                    player.bottom_y(),
                    "Ztarred!".to_string(),
                    (-(VELJUMP as f64) * 1.5) as f32,
                ));
            }

            g.eyecandy[2].emplace(EC_SingleAnimation::new(
                Ptr::from_mut(&mut g.rm.spr_fireballexplosion),
                (player.center_x() as i32 - 16) as i16,
                (player.center_y() as i32 - 16) as i16,
                3,
                8,
            ));
            if_sound_on_play(g, &mut gp.rm.sfx_transform);

            oldstar
        }
    }

    fn rebalance_multi_stars(&mut self, g: &mut Game) {
        let mut rankedPlayers: [Ptr<CPlayer>; 4] = [Ptr::null(); 4];
        let fReverseScoring = self.fReverseScoring;
        let iNumPlayers: i16 = self.get_score_ranked_player_list(g, &mut rankedPlayers, fReverseScoring);

        let mut iStar: i16 = 0;
        while (iStar as i32) < iNumPlayers as i32 - 1 {
            self.starPlayer[iStar as usize] = rankedPlayers[iStar as usize];
            let iColor = self.starPlayer[iStar as usize].get_color_id();
            self.starItem[iStar as usize].set_player_color(iColor);
            self.starItem[iStar as usize].place_star(g);
            iStar += 1;
        }
    }
}

impl CGameModeTrait for CGM_Star {
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self, g: &mut Game) {
        cgm_star_init(g, self)
    }
    fn think(&mut self, g: &mut Game) {
        cgm_star_think(g, self)
    }
    fn draw_foreground(&mut self, g: &mut Game) {
        if self.fDisplayTimer {
            self.gameClock.draw(g);
        }
    }
    fn playerkilledplayer(&mut self, g: &mut Game, _inflictor: Ptr<CPlayer>, _other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        PlayerKillType::Normal
    }
    fn playerkilledself(&mut self, g: &mut Game, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgamemode_playerkilledself(self, player, style)
    }
    fn playerextraguy(&mut self, g: &mut Game, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            if iType == 5 {
                player.score().adjust_score(g, if self.fReverseScoring { -1 } else { 1 });
            } else {
                self.gameClock.add_time((iType as i32 * 10) as i16);
            }
        }
    }
}

fn star_time(g: &mut Game) -> i16 {
    unsafe {
        if g.game_values.gamemodesettings.star.time < 1 {
            30
        } else {
            g.game_values.gamemodesettings.star.time
        }
    }
}

pub fn cgm_star_init(g: &mut Game, this: &mut CGM_Star) {
        let mut gp = g.ptr();
        let mut gp = g.ptr();
    cgm_timelimit_init(g, &mut this.cgm_time_limit);

    this.fDisplayTimer = true;

    this.gameClock.init(g, star_time(gp.get()), true);

    unsafe {
        this.iCurrentModeType = g.game_values.gamemodesettings.star.shine;
        if this.iCurrentModeType == StarStyle::Random {
            this.iCurrentModeType = StarStyle::from_u8(RANDOM_INT(gp.get(), 3) as u8);
        }

        this.fReverseScoring = this.goal == -1;

        //Set initial scores
        for iScore in 0..game().score_cnt {
            if this.fReverseScoring {
                gp.score[iScore as usize].set_score(g, 0);
            } else {
                gp.score[iScore as usize].set_score(g, this.goal);
            }
        }
    }

    for iStar in 0..3usize {
        this.starItem[iStar] = Ptr::null();
        this.starPlayer[iStar] = Ptr::null();
    }

    this.setup_mode(g);
}

fn star_needs_reassign(this: &CGM_Star) -> bool {
    unsafe {
        if this.iCurrentModeType == StarStyle::Multi {
            (0..game().players.len().saturating_sub(1)).any(|i| this.starPlayer[i].is_null())
        } else {
            this.starPlayer[0].is_null()
        }
    }
}

fn reassign(g: &mut Game, this: &mut CGM_Star) {
    unsafe {
        if this.iCurrentModeType == StarStyle::Multi {
            let mut iStar1: usize = 0;
            while iStar1 + 1 < g.players.len() {
                //If we're missing a star player, then reassign them all
                if this.starPlayer[iStar1].is_null() {
                    this.rebalance_multi_stars(g);
                    break;
                }
                iStar1 += 1;
            }
        } else if this.starPlayer[0].is_null() {
            let fGetHighest = !this.fReverseScoring && g.game_values.gamemodesettings.star.shine == StarStyle::Ztar;
            this.starPlayer[0] = this.get_highest_score_player(g, fGetHighest);
            this.starItem[0].place_star(g);
        }
    }
}

fn time_out(g: &mut Game, this: &mut CGM_Star) {
        let mut gp = g.ptr();
    unsafe {
        this.gameClock.set_time(star_time(g));
        if_sound_on_play(g, &mut gp.rm.sfx_thunder);

        if this.iCurrentModeType == StarStyle::Ztar {
            if g.score[this.starPlayer[0].get_team_id() as usize].score > 1 || this.fReverseScoring {
                this.starPlayer[0].kill_player_map_hazard(g, true, KillStyle::Environment, false, -1);
            }

            if this.fReverseScoring {
                this.starPlayer[0].score().adjust_score(g, 1);
            } else {
                this.starPlayer[0].score().adjust_score(g, -1);

                if this.starPlayer[0].score().score <= 0 {
                    this.fDisplayTimer = !remove_team(g, this.starPlayer[0].get_team_id());
                    this.starPlayer[0] = Ptr::null();
                }
            }

            let fGetHighest = !this.fReverseScoring;
            this.starPlayer[0] = this.get_highest_score_player(g, fGetHighest);
            this.starItem[0].place_star(g);
        } else if this.iCurrentModeType == StarStyle::Shine {
            for i in 0..game().players.len() {
                let mut player = g.players[i];
                if this.starPlayer[0].get_team_id() == player.get_team_id() {
                    continue;
                }

                //Let the cleanup function remove the player on the last kill
                if g.score[player.get_team_id() as usize].score > 1 || this.fReverseScoring {
                    player.kill_player_map_hazard(g, true, KillStyle::Environment, false, -1);
                }
            }

            if this.fReverseScoring {
                this.starPlayer[0].score().adjust_score(g, 1);
            } else {
                for iTeam in 0..game().score_cnt {
                    if this.starPlayer[0].get_team_id() == iTeam {
                        continue;
                    }

                    gp.score[iTeam as usize].adjust_score(g, -1);

                    if g.score[iTeam as usize].score <= 0 {
                        this.fDisplayTimer = !remove_team(g, iTeam);
                    }
                }
            }

            this.starPlayer[0] = this.get_highest_score_player(g, false);
            this.starItem[0].place_star(g);
        } else if this.iCurrentModeType == StarStyle::Multi {
            for iPlayer in 0..game().players.len() {
                let mut fFound = false;
                let mut iStar: usize = 0;
                while iStar + 1 < g.players.len() {
                    if this.starPlayer[iStar] == g.players[iPlayer] {
                        fFound = true;
                        break;
                    }
                    iStar += 1;
                }

                if fFound {
                    continue;
                }

                let mut p = g.players[iPlayer];
                if g.score[p.get_team_id() as usize].score > 1 || this.fReverseScoring {
                    p.kill_player_map_hazard(g, true, KillStyle::Environment, false, -1);
                }

                let mut fNeedRebalance = true;
                let mut p = g.players[iPlayer];
                if this.fReverseScoring {
                    p.score().adjust_score(g, 1);
                } else {
                    p.score().adjust_score(g, -1);

                    if p.score().score <= 0 {
                        this.fDisplayTimer = !remove_team(g, p.get_team_id());

                        if g.game_values.gamemodesettings.star.shine != StarStyle::Random {
                            this.setup_mode(g);
                            fNeedRebalance = false;
                        }
                    }
                }

                if g.game_values.gamemodesettings.star.shine != StarStyle::Random && fNeedRebalance {
                    this.rebalance_multi_stars(g);
                }

                break;
            }
        }

        //Play warning sound if needed
        if !this.fReverseScoring && !this.playedwarningsound {
            let mut countscore: i16 = 0;
            for j in 0..game().score_cnt {
                for k in 0..game().score_cnt {
                    if j == k {
                        continue;
                    }

                    countscore = (countscore as i32 + g.score[k as usize].score as i32) as i16;
                }

                if countscore <= 1 {
                    this.playwarningsound(g);
                    break;
                }

                countscore = 0;
            }
        }

        //If random game, then choose a new game type
        if g.game_values.gamemodesettings.star.shine == StarStyle::Random && this.fDisplayTimer {
            this.iCurrentModeType = StarStyle::from_u8(RANDOM_INT(gp.get(), 3) as u8);
            this.setup_mode(g);
        }
    }
}

fn star_mode() -> Option<&'static mut CGM_Star> {
    unsafe { game().game_values.gamemode.get().as_any().downcast_mut::<CGM_Star>() }
}

pub fn net_reassign(g: &mut Game) {
    if let Some(this) = star_mode() {
        if star_needs_reassign(this) {
            reassign(g, this);
        }
    }
}

pub fn net_timeout(g: &mut Game) {
    if let Some(this) = star_mode() {
        if !star_needs_reassign(this) {
            time_out(g, this);
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

pub fn cgm_star_think(g: &mut Game, this: &mut CGM_Star) {
        let mut gp = g.ptr();
    if this.gameover {
        this.displayplayertext(g);
        return;
    }

    unsafe {
        //Make sure there is a star player(s)
        if star_needs_reassign(this) && !net_random::event(g, Ev::StarReassign, &[]) {
            reassign(g, this);
        }

        //Count down the game time
        let iTime: i16 = this.gameClock.run_clock();
        if iTime <= 5 && iTime > 0 {
            if_sound_on_play(g, &mut gp.rm.sfx_starwarning);
        }

        //If the game time ran out, somebody needs to die and scores changed
        if iTime == 0 && !net_random::event(g, Ev::StarTimeout, &[]) {
            time_out(g, this);
        }
    }
}
