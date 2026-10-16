//! Port of src/smw/gamemodes/GameMode.cpp

use crate::game::Game;
use crate::common::eyecandy::EC_Announcement;
use crate::common::game_mode::*;
use crate::common::game_values::if_sound_on_play;
use crate::common::match_types::MatchType;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::{RANDOM_BOOL, RANDOM_INT};
use crate::globals::*;
use crate::smw::net_random::{self, Ev};
use crate::smw::objects::powerup::pu_treasure_chest_bonus::PU_TreasureChestBonus;
use crate::smw::player::{CPlayer, PlayerState};

/// Implements `gm`, `gm_mut` and `as_any` of `CGameModeTrait` for a type that derefs (possibly
/// through several bases) to `CGameMode`.
#[macro_export]
macro_rules! impl_cgamemode_plumbing {
    () => {
        fn gm(&self) -> &$crate::common::game_mode::CGameMode {
            self
        }
        fn gm_mut(&mut self) -> &mut $crate::common::game_mode::CGameMode {
            self
        }
        fn as_any(&mut self) -> &mut dyn ::std::any::Any {
            self
        }
    };
}

pub fn remove_players_but_team(g: &mut Game, teamid: i16) {
    if !net_random::event(g, Ev::RemoveButTeam, &[teamid as i32]) {
        remove_players_but_team_now(g, teamid);
    }
}

pub fn remove_players_but_team_now(g: &mut Game, teamid: i16) {
    unsafe {
        for i in 0..game().players.len() {
            let mut player = g.players[i];
            if player.get_team_id() != teamid {
                player.state = PlayerState::Dead;
            }
        }
    }
}

pub fn remove_players_but_highest_scoring(g: &mut Game) {
    if !net_random::event(g, Ev::RemoveButHighest, &[]) {
        remove_players_but_highest_scoring_now(g);
    }
}

pub fn remove_players_but_highest_scoring_now(g: &mut Game) {
    unsafe {
        let mut iMaxScore: i16 = -1;

        //Figure out what the maximum score is
        for iScore in 0..game().score_cnt {
            if g.score[iScore as usize].score > iMaxScore {
                iMaxScore = g.score[iScore as usize].score;
            }
        }

        //Remove all players that don't have that max score
        for i in 0..game().players.len() {
            let mut player = g.players[i];
            if player.score().score < iMaxScore {
                player.state = PlayerState::Dead;
            }
        }
    }
}

pub fn setup_score_board(g: &mut Game, fOrderMatters: bool) {
    if !net_random::event(g, Ev::ScoreBoard, &[fOrderMatters as i32]) {
        setup_score_board_now(g, fOrderMatters);
    }
}

pub fn setup_score_board_now(g: &mut Game, fOrderMatters: bool) {
        let mut gp = g.ptr();
    unsafe {
        let mut doneWithScore: [bool; 4] = [false, false, false, false];

        let mut oldmax: i16;
        let mut max: i16 = -1;
        for i in 0..game().score_cnt {
            oldmax = max;
            max = -1;

            for j in 0..game().score_cnt {
                if !doneWithScore[j as usize] {
                    let sj = g.score[j as usize];
                    //The boxes minigame doesn't use "score" it uses "subscore[0]" to determine the winner
                    if g.game_values.gamemode.gamemode == game_mode_boxes_minigame {
                        if max == -1
                            || sj.subscore[0] > g.score[max as usize].subscore[0]
                            || (sj.subscore[0] == g.score[max as usize].subscore[0] && sj.score > g.score[max as usize].score)
                        {
                            //or it is tied but they died later in the game
                            max = j;
                        }
                    } else {
                        //If this player's score is bigger
                        if max == -1
                            || sj.score > g.score[max as usize].score
                            || (sj.score == g.score[max as usize].score && sj.order > g.score[max as usize].order)
                        {
                            //or it is tied but they died later in the game
                            max = j;
                        }
                    }
                }
            }

            g.score[max as usize].displayorder = i;
            g.score[max as usize].place = i;

            if !fOrderMatters && i > 0 {
                if g.game_values.gamemode.gamemode == game_mode_boxes_minigame {
                    if g.score[oldmax as usize].subscore[0] == g.score[max as usize].subscore[0]
                        && g.score[oldmax as usize].score == g.score[max as usize].score
                    {
                        g.score[max as usize].place = g.score[oldmax as usize].place;
                    }
                } else if g.score[oldmax as usize].score == g.score[max as usize].score {
                    g.score[max as usize].place = g.score[oldmax as usize].place;
                }
            }

            doneWithScore[max as usize] = true; //this is the next biggest score - it doesn't belong to the remaining scores from now on
        }

        //Add the treasure chests to the map in world mode if there were any awards for winning this match
        if g.game_values.matchtype == MatchType::World && g.game_values.gamemode.winningteam > -1 && g.game_values.gamemode.gamemode != game_mode_bonus {
            let tourStop = g.game_values.tourstops[g.game_values.tourstopcurrent];
            let iNumBonuses: i16 = tourStop.iNumBonuses;

            for iBonus in 0..iNumBonuses {
                if tourStop.wsbBonuses[iBonus as usize].iWinnerPlace == 0 {
                    let iBonusType: i16 = tourStop.wsbBonuses[iBonus as usize].iBonus;
                    let obj = Ptr::new_box(PU_TreasureChestBonus::new(g, Ptr::from_mut(&mut gp.get().rm.spr_bonuschest), 1, 0, 30, 30, 1, 1, iBonusType));
                    gp.get().objectcontainer[0].add(g, obj);
                    g.game_values.flags.noexittimer = 0;
                    g.game_values.flags.noexit = false;
                }
            }
        }
    }
}

pub fn show_score_board(g: &mut Game) {
    if !net_random::event(g, Ev::ShowScoreBoard, &[]) {
        show_score_board_now(g);
    }
}

pub fn show_score_board_now(g: &mut Game) {
    let mut gp = g.ptr();
    unsafe {
        g.game_values.flags.showscoreboard = true;

        let mut iScoreboardElementHeight: i16 = 45;
        if g.game_values.gamemode.gamemode == game_mode_health
            || g.game_values.gamemode.gamemode == game_mode_collection
            || g.game_values.gamemode.gamemode == game_mode_boxes_minigame
        {
            iScoreboardElementHeight = 63;
        }

        for i in 0..game().score_cnt {
            let mut s = g.score[i as usize];
            s.destx = (309 - 34 * g.game_values.teamcounts[i as usize] as i32) as i16;
            s.desty = (s.displayorder as i32 * iScoreboardElementHeight as i32 + 140) as i16;
        }

        if g.game_values.music {
            gp.rm.sfx_invinciblemusic.stop(g);
            gp.rm.sfx_timewarning.stop(g);
            gp.rm.sfx_slowdownmusic.stop(g);

            gp.rm.backgroundmusic[1].play(g, true, false);
        }
    }
}

//Returns true if all but one team is dead
pub fn remove_team(g: &mut Game, teamid: i16) -> bool {
    match crate::smw::net_outcomes::remove_team_event(g, teamid) {
        Some(result) => result,
        None => remove_team_now(g, teamid),
    }
}

pub fn remove_team_now(g: &mut Game, teamid: i16) -> bool {
    let mut gp = g.ptr();
    unsafe {
        //If we have already removed this team then return
        if g.score[teamid as usize].order > -1 {
            return g.game_values.flags.teamdeadcounter as i32 == g.score_cnt as i32 - 1;
        }

        //kill all players on the dead team
        let mut iAnnouncementColor: i16 = -1;
        for i in 0..game().players.len() {
            let mut player = g.players[i];
            if player.get_team_id() == teamid {
                if iAnnouncementColor == -1 {
                    iAnnouncementColor = player.get_color_id();
                }

                player.state = PlayerState::Dead;
            }
        }

        g.score[teamid as usize].order = g.game_values.flags.teamdeadcounter;
        g.game_values.flags.teamdeadcounter += 1;

        //Announce that a team was removed
        if g.game_values.deadteamnotice && (g.game_values.flags.teamdeadcounter as i32) < g.score_cnt as i32 - 1 {
            g.eyecandy[2].emplace(EC_Announcement::new(
                Ptr::from_mut(&mut g.rm.game_font_large),
                Ptr::from_mut(&mut g.rm.spr_announcementicons),
                "Team Removed!".to_string(),
                iAnnouncementColor,
                90,
                200,
            ));
            gp.rm.sfx_announcer[(iAnnouncementColor as i32 + 16) as usize].play(g);
        }

        g.game_values.flags.teamdeadcounter as i32 == g.score_cnt as i32 - 1
    }
}

impl CGameMode {
    pub fn new() -> Self {
        let mut this = CGameMode {
            winningteam: -1,
            gameover: false,
            gamemode: game_mode_frag,
            playedwarningsound: false,
            goal: 0,
            szModeName: String::new(),
            szGoalName: String::new(),
            modeOptions: Default::default(),
            fReverseScoring: false,
            _alias: Aliased::new(),
        };
        this.setup_mode_strings("Free Play", "Frags", 5);
        this.fReverseScoring = false;
        this
    }

    pub fn displayplayertext(&mut self, g: &mut Game) {
        if self.winningteam > -1 {
            unsafe {
                for i in 0..game().players.len() {
                    let mut player = g.players[i];
                    if player.get_team_id() == self.winningteam {
                        player.spawn_text(g, "Winner!");
                    }
                }
            }
        }
    }

    pub fn playwarningsound(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if self.playedwarningsound {
            return;
        }

        self.playedwarningsound = true;
        unsafe {
            gp.rm.sfx_invinciblemusic.stop(g);

            if g.game_values.music && g.game_values.sound {
                gp.rm.backgroundmusic[0].stop(g);
            }

            if_sound_on_play(g, &mut gp.rm.sfx_timewarning);
        }
    }

    pub fn setup_mode_strings(&mut self, szMode: &str, szGoal: &str, iGoalSpacing: i16) {
        self.szModeName = szMode.to_string();
        self.szGoalName = szGoal.to_string();

        for iMode in 0..GAMEMODE_NUM_OPTIONS as i16 {
            if iMode as usize == GAMEMODE_NUM_OPTIONS - 1 {
                self.modeOptions[iMode as usize].iValue = -1;
                self.modeOptions[iMode as usize].szName = "Unlimited".to_string();
            } else {
                self.modeOptions[iMode as usize].iValue = ((iMode as i32 + 1) * iGoalSpacing as i32) as i16;
                self.modeOptions[iMode as usize].szName = self.modeOptions[iMode as usize].iValue.to_string();
            }
        }
    }

    pub fn get_highest_score_player(&mut self, g: &mut Game, fGetHighest: bool) -> Ptr<CPlayer> {
        let mut gp = g.ptr();
        unsafe {
            let mut count: i16 = 1;
            let mut tiedplayers: [i16; 4] = [0; 4];
            tiedplayers[0] = 0;

            //Find the first non-dead player and use them for the first player to compare to
            let mut j: usize = 0;
            while j < g.players.len() {
                if !g.players[j].isdead() {
                    count = 1;
                    tiedplayers[0] = j as i16;
                    break;
                }
                j += 1;
            }

            //Loop through all players, comparing scores to find the highest/lowest
            let mut i = j + 1;
            while i < g.players.len() {
                if !g.players[i].isdead() {
                    let si = g.players[i].score().score;
                    let s0 = g.players[tiedplayers[0] as usize].score().score;
                    if (!fGetHighest && si < s0) || (fGetHighest && si > s0) {
                        count = 1;
                        tiedplayers[0] = i as i16;
                    } else if si == s0 {
                        tiedplayers[count as usize] = i as i16;
                        count += 1;
                    }
                }
                i += 1;
            }

            g.players[tiedplayers[RANDOM_INT(gp.get(), count as i32) as usize] as usize]
        }
    }

    //Returns number of players in list
    pub fn get_score_ranked_player_list(&mut self, g: &mut Game, outPlayers: &mut [Ptr<CPlayer>; 4], fGetHighest: bool) -> i16 {
        let mut gp = g.ptr();
        unsafe {
            let mut iNumPlayersInList: i16 = 0;

            for i in 0..game().players.len() {
                let player = g.players[i];
                if player.isdead() {
                    continue;
                }

                outPlayers[iNumPlayersInList as usize] = player;
                iNumPlayersInList += 1;
            }

            //Bubble sort players in to score order
            let mut fNeedSwap = true;

            while fNeedSwap {
                fNeedSwap = false;
                let mut iRandom: i16 = 0;
                let mut iIndex: i16 = 0;
                while (iIndex as i32) < iNumPlayersInList as i32 - 1 {
                    let a = outPlayers[iIndex as usize].score().score;
                    let b = outPlayers[iIndex as usize + 1].score().score;
                    let swap = (fGetHighest && a < b) || (!fGetHighest && a > b) || (a == b && RANDOM_BOOL(gp.get()) && {
                        let r = iRandom < 5;
                        iRandom += 1;
                        r
                    });
                    if swap {
                        outPlayers.swap(iIndex as usize, iIndex as usize + 1);

                        fNeedSwap = true;
                    }
                    iIndex += 1;
                }
            }

            iNumPlayersInList
        }
    }

    pub fn get_closest_goal(&mut self, iGoal: i16) -> i16 {
        let mut iDifference: i16 = 16000;
        let mut iOptionValue: i16 = 0;

        for iOption in 0..GAMEMODE_NUM_OPTIONS - 1 {
            let iDiff: i16 = (self.modeOptions[iOption].iValue as i32 - iGoal as i32).abs() as i16;
            if iDiff < iDifference {
                iOptionValue = self.modeOptions[iOption].iValue;
                iDifference = iDiff;
            }
        }

        iOptionValue
    }
}

impl Default for CGameMode {
    fn default() -> Self {
        CGameMode::new()
    }
}

pub fn cgamemode_init<T: CGameModeTrait + ?Sized>(g: &mut Game, this: &mut T) {
        let mut gp = g.ptr();
    let this = this.gm_mut();
    if this.goal == 1000 {
        this.goal = 999; //Cap goal for 3 digit scoreboard
    }

    this.winningteam = -1;
    this.gameover = false;
    this.playedwarningsound = false;

    unsafe {
        for iScore in 0..game().score_cnt {
            gp.score[iScore as usize].set_score(g, 0);
        }
    }
}

pub fn cgamemode_think<T: CGameModeTrait + ?Sized>(g: &mut Game, this: &mut T) {
        let mut gp = g.ptr();
    unsafe {
        if g.netplay.active {
            for k in 0..game().players.len() {
                if g.netplay.player_disconnected[k] {
                    gp.players[k].spawn_text(g, "Disconnected!");
                }
            }
        }
    }

    if this.gm().gameover {
        this.gm_mut().displayplayertext(g);
    }
}

pub fn cgamemode_playerkilledplayer<T: CGameModeTrait + ?Sized>(g: &mut Game, this: &mut T, mut inflictor: Ptr<CPlayer>, other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
    //Penalize killing your team mates
    if !this.gm().gameover {
        if inflictor.get_team_id() == other.get_team_id() {
            inflictor.score().adjust_score(g, -1);
        } else {
            inflictor.score().adjust_score(g, 1);
        }
    }

    PlayerKillType::Normal
}

pub fn cgamemode_playerkilledself<T: CGameModeTrait + ?Sized>(_this: &mut T, mut player: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
    if player.is_bobomb() {
        player.set_corpse_type(2); //flag to use bobomb corpse sprite
    }

    PlayerKillType::Normal
}

pub fn cgamemode_playerextraguy<T: CGameModeTrait + ?Sized>(g: &mut Game, this: &mut T, mut player: Ptr<CPlayer>, iType: i16) {
    if !this.gm().gameover {
        player.score().adjust_score(g, iType);
    }
}
