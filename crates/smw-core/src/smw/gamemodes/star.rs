//! Port of src/smw/gamemodes/Star.cpp

use crate::smw::object_container::ObjectId;
use crate::smw::players::PlayerId;
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

use crate::smw::objects::carriable::co_star::CO_Star;
use crate::smw::objects::moving::mo_carried_object::MO_CarriedObjectTrait;
use crate::smw::player::CPlayer;

//Star mode - shared timer ticks down and the players must pass around
//a hot potato.  When the timer hits zero, the player with the star
//loses a point.
#[derive(Clone)]
pub struct CGM_Star {
    pub cgm_time_limit: CGM_TimeLimit,

    pub starItem: [ObjectId; 3],
    pub starPlayer: [PlayerId; 3],
    pub iCurrentModeType: StarStyle,
    pub fDisplayTimer: bool,
}
impl_base!(CGM_Star => cgm_time_limit: CGM_TimeLimit);

impl CGM_Star {
    pub fn new() -> Self {
        let mut this = CGM_Star {
            cgm_time_limit: CGM_TimeLimit::new(),
            starItem: [ObjectId::NONE; 3],
            starPlayer: [PlayerId::NONE; 3],
            iCurrentModeType: StarStyle::Ztar,
            fDisplayTimer: false,
        };
        this.goal = 5;
        this.gamemode = game_mode_star;
        this.setup_mode_strings("Star", "Lives", 1);
        this.iCurrentModeType = StarStyle::Ztar;
        this
    }

    pub fn getstarplayer(&mut self, id: i16) -> PlayerId {
        self.starPlayer[id as usize]
    }

    pub fn getcurrentmodetype(&self) -> StarStyle {
        self.iCurrentModeType
    }

    fn setup_mode(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        //Clean up old stars
        for iStar in 0..3usize {
            if !self.starItem[iStar].is_none() {
                if let Some(star) = g.object_as::<CO_Star>(self.starItem[iStar]) {
                    MO_CarriedObjectTrait::drop(star, g);
                    star.dead = true;
                }
                self.starItem[iStar] = ObjectId::NONE;
            }

            self.starPlayer[iStar] = PlayerId::NONE;
        }

        //If multi star, add more stars
        if self.iCurrentModeType == StarStyle::Multi {
            let mut rankedPlayers: [PlayerId; 4] = [PlayerId::NONE; 4];
            let fReverseScoring = self.fReverseScoring;
            let iNumPlayers: i16 = self.get_score_ranked_player_list(g, &mut rankedPlayers, fReverseScoring);

            let mut iStar: i16 = 0;
            while (iStar as i32) < iNumPlayers as i32 - 1 {
                self.starPlayer[iStar as usize] = rankedPlayers[iStar as usize];

                let mut star = Ptr::new_box(CO_Star::new(g, gp.rm.spr_star, 1, iStar));
                star.set_player_color(g.player(self.starPlayer[iStar as usize]).get_color_id());
                self.starItem[iStar as usize] = gp.objectcontainer[1].add_id(g, star);
                iStar += 1;
            }
        } else {
            //otherwise, add just a single star
            let fGetHighest = !self.fReverseScoring && self.iCurrentModeType == StarStyle::Ztar;
            self.starPlayer[0] = self.get_highest_score_player(g, fGetHighest);

            let star = Ptr::new_box(CO_Star::new(g, gp.rm.spr_star, if self.iCurrentModeType == StarStyle::Ztar { 0 } else { 1 }, 0));
            self.starItem[0] = gp.objectcontainer[1].add_id(g, star);
        }
    }

    pub fn isplayerstar(&mut self, g: &mut Game, player: PlayerId) -> bool {
        let mut iPlayer: i16 = 0;
        while (iPlayer as usize) < g.players.len().wrapping_sub(1) {
            if self.starPlayer[iPlayer as usize] == player {
                return true;
            }
            iPlayer += 1;
        }

        false
    }

    pub fn swapplayer(&mut self, g: &mut Game, id: i16, player: PlayerId) -> PlayerId {
        let mut gp = g.ptr();
        let mut oldstar: PlayerId = PlayerId::NONE;
        if !self.starPlayer[id as usize].is_none() {
            oldstar = self.starPlayer[id as usize];
            g.player(oldstar).shield().turn_on(g);
            g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, 
                g.rm.spr_fireballexplosion,
                (g.player(oldstar).center_x() as i32 - 16) as i16,
                (g.player(oldstar).center_y() as i32 - 16) as i16,
                3,
                8,
            ));
        }

        self.starPlayer[id as usize] = player;

        if self.iCurrentModeType == StarStyle::Multi {
            let iColor = g.player(self.starPlayer[id as usize]).get_color_id();
            star_item(g, self.starItem[id as usize]).set_player_color(iColor);
        }

        if star_item(g, self.starItem[id as usize]).get_type() == 1 {
            let text = if self.iCurrentModeType == StarStyle::Multi { "Star Get!" } else { "Shine Get!" };
            g.eyecandy[2].emplace(EC_GravText::new(
                Ptr::from_mut(&mut g.rm.game_font_large),
                g.player(player).center_x(),
                g.player(player).bottom_y(),
                text.to_string(),
                (-(VELJUMP as f64) * 1.5) as f32,
            ));
        } else {
            g.eyecandy[2].emplace(EC_GravText::new(
                Ptr::from_mut(&mut g.rm.game_font_large),
                g.player(player).center_x(),
                g.player(player).bottom_y(),
                "Ztarred!".to_string(),
                (-(VELJUMP as f64) * 1.5) as f32,
            ));
        }

        g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, 
            g.rm.spr_fireballexplosion,
            (g.player(player).center_x() as i32 - 16) as i16,
            (g.player(player).center_y() as i32 - 16) as i16,
            3,
            8,
        ));
        if_sound_on_play(g, &mut gp.rm.sfx_transform);

        oldstar
    }

    fn rebalance_multi_stars(&mut self, g: &mut Game) {
        let mut rankedPlayers: [PlayerId; 4] = [PlayerId::NONE; 4];
        let fReverseScoring = self.fReverseScoring;
        let iNumPlayers: i16 = self.get_score_ranked_player_list(g, &mut rankedPlayers, fReverseScoring);

        let mut iStar: i16 = 0;
        while (iStar as i32) < iNumPlayers as i32 - 1 {
            self.starPlayer[iStar as usize] = rankedPlayers[iStar as usize];
            let iColor = g.player(self.starPlayer[iStar as usize]).get_color_id();
            star_item(g, self.starItem[iStar as usize]).set_player_color(iColor);
            star_item(g, self.starItem[iStar as usize]).place_star(g);
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
    fn playerkilledplayer(&mut self, g: &mut Game, _inflictor: PlayerId, _other: PlayerId, _style: KillStyle) -> PlayerKillType {
        PlayerKillType::Normal
    }
    fn playerkilledself(&mut self, g: &mut Game, player: PlayerId, style: KillStyle) -> PlayerKillType {
        cgamemode_playerkilledself(g, self, player, style)
    }
    fn playerextraguy(&mut self, g: &mut Game, mut player: PlayerId, iType: i16) {
        if !self.gameover {
            if iType == 5 {
                g.player(player).score(g).adjust_score(g, if self.fReverseScoring { -1 } else { 1 });
            } else {
                self.gameClock.add_time((iType as i32 * 10) as i16);
            }
        }
    }
}

fn star_time(g: &mut Game) -> i16 {
    if g.game_values.gamemodesettings.star.time < 1 {
        30
    } else {
        g.game_values.gamemodesettings.star.time
    }
}

pub fn cgm_star_init(g: &mut Game, this: &mut CGM_Star) {
        let mut gp = g.ptr();
    cgm_timelimit_init(g, &mut this.cgm_time_limit);

    this.fDisplayTimer = true;

    this.gameClock.init(g, star_time(gp.get()), true);

    this.iCurrentModeType = g.game_values.gamemodesettings.star.shine;
    if this.iCurrentModeType == StarStyle::Random {
        this.iCurrentModeType = StarStyle::from_u8(RANDOM_INT(gp.get(), 3) as u8);
    }

    this.fReverseScoring = this.goal == -1;

    //Set initial scores
    for iScore in 0..g.score_cnt {
        if this.fReverseScoring {
            gp.score[iScore as usize].set_score(g, 0);
        } else {
            gp.score[iScore as usize].set_score(g, this.goal);
        }
    }

    for iStar in 0..3usize {
        this.starItem[iStar] = ObjectId::NONE;
        this.starPlayer[iStar] = PlayerId::NONE;
    }

    this.setup_mode(g);
}

fn star_needs_reassign(g: &mut Game, this: &CGM_Star) -> bool {
    let mut gp = g.ptr();
    if this.iCurrentModeType == StarStyle::Multi {
        (0..gp.players.len().saturating_sub(1)).any(|i| this.starPlayer[i].is_none())
    } else {
        this.starPlayer[0].is_none()
    }
}

fn reassign(g: &mut Game, this: &mut CGM_Star) {
    if this.iCurrentModeType == StarStyle::Multi {
        let mut iStar1: usize = 0;
        while iStar1 + 1 < g.players.len() {
            //If we're missing a star player, then reassign them all
            if this.starPlayer[iStar1].is_none() {
                this.rebalance_multi_stars(g);
                break;
            }
            iStar1 += 1;
        }
    } else if this.starPlayer[0].is_none() {
        let fGetHighest = !this.fReverseScoring && g.game_values.gamemodesettings.star.shine == StarStyle::Ztar;
        this.starPlayer[0] = this.get_highest_score_player(g, fGetHighest);
        star_item(g, this.starItem[0]).place_star(g);
    }
}

fn time_out(g: &mut Game, this: &mut CGM_Star) {
        let mut gp = g.ptr();
    this.gameClock.set_time(star_time(g));
    if_sound_on_play(g, &mut gp.rm.sfx_thunder);

    if this.iCurrentModeType == StarStyle::Ztar {
        if g.score[g.player(this.starPlayer[0]).get_team_id() as usize].score > 1 || this.fReverseScoring {
            g.player(this.starPlayer[0]).kill_player_map_hazard(g, true, KillStyle::Environment, false, -1);
        }

        if this.fReverseScoring {
            g.player(this.starPlayer[0]).score(g).adjust_score(g, 1);
        } else {
            g.player(this.starPlayer[0]).score(g).adjust_score(g, -1);

            if g.player(this.starPlayer[0]).score(g).score <= 0 {
                this.fDisplayTimer = !remove_team(g, g.player(this.starPlayer[0]).get_team_id());
                this.starPlayer[0] = PlayerId::NONE;
            }
        }

        let fGetHighest = !this.fReverseScoring;
        this.starPlayer[0] = this.get_highest_score_player(g, fGetHighest);
        star_item(g, this.starItem[0]).place_star(g);
    } else if this.iCurrentModeType == StarStyle::Shine {
        for i in 0..g.players.len() {
            let mut player = g.players[i];
            if g.player(this.starPlayer[0]).get_team_id() == g.player(player).get_team_id() {
                continue;
            }

            //Let the cleanup function remove the player on the last kill
            if g.score[g.player(player).get_team_id() as usize].score > 1 || this.fReverseScoring {
                g.player(player).kill_player_map_hazard(g, true, KillStyle::Environment, false, -1);
            }
        }

        if this.fReverseScoring {
            g.player(this.starPlayer[0]).score(g).adjust_score(g, 1);
        } else {
            for iTeam in 0..g.score_cnt {
                if g.player(this.starPlayer[0]).get_team_id() == iTeam {
                    continue;
                }

                gp.score[iTeam as usize].adjust_score(g, -1);

                if g.score[iTeam as usize].score <= 0 {
                    this.fDisplayTimer = !remove_team(g, iTeam);
                }
            }
        }

        this.starPlayer[0] = this.get_highest_score_player(g, false);
        star_item(g, this.starItem[0]).place_star(g);
    } else if this.iCurrentModeType == StarStyle::Multi {
        for iPlayer in 0..g.players.len() {
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
            if g.score[g.player(p).get_team_id() as usize].score > 1 || this.fReverseScoring {
                g.player(p).kill_player_map_hazard(g, true, KillStyle::Environment, false, -1);
            }

            let mut fNeedRebalance = true;
            let mut p = g.players[iPlayer];
            if this.fReverseScoring {
                g.player(p).score(g).adjust_score(g, 1);
            } else {
                g.player(p).score(g).adjust_score(g, -1);

                if g.player(p).score(g).score <= 0 {
                    this.fDisplayTimer = !remove_team(g, g.player(p).get_team_id());

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
        for j in 0..g.score_cnt {
            for k in 0..g.score_cnt {
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

fn star_mode(g: &mut Game) -> Option<&'static mut CGM_Star> {
    let mut gp = g.ptr();
    gp.mode().get().as_any().downcast_mut::<CGM_Star>()
}

/// Replay-harness `T` record (docs/guides/REPLAY.md).
pub fn harness_record(g: &mut Game) -> Option<String> {
    star_mode(g).map(|this| {
        let ids: Vec<String> = this.starPlayer.iter().map(|p| if p.is_none() { "-1".to_string() } else { g.player(*p).globalID.to_string() }).collect();
        format!("T type={} holders={}", this.iCurrentModeType as i32, ids.join(","))
    })
}

pub fn cgm_star_think(g: &mut Game, this: &mut CGM_Star) {
        let mut gp = g.ptr();
    if this.gameover {
        this.displayplayertext(g);
        return;
    }

    //Make sure there is a star player(s)
    if star_needs_reassign(g, this) {
        reassign(g, this);
    }

    //Count down the game time
    let iTime: i16 = this.gameClock.run_clock();
    if iTime <= 5 && iTime > 0 {
        if_sound_on_play(g, &mut gp.rm.sfx_starwarning);
    }

    //If the game time ran out, somebody needs to die and scores changed
    if iTime == 0 {
        time_out(g, this);
    }
}

/// Stars live from `setup_mode` to the next one.
fn star_item(g: &Game, id: ObjectId) -> &'static mut CO_Star {
    g.object_as::<CO_Star>(id).unwrap_or_else(|| panic!("no star {:?}", id))
}
