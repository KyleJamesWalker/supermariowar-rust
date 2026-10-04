//! Port of src/smw/gamemodes/GameMode.cpp

use crate::common::eyecandy::EC_Announcement;
use crate::common::game_mode::*;
use crate::common::game_values::if_sound_on_play;
use crate::common::match_types::MatchType;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::{RANDOM_BOOL, RANDOM_INT};
use crate::globals::*;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::{players, score, score_cnt};
use crate::smw::net::netplay;
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

pub fn remove_players_but_team(teamid: i16) {
    if !net_random::event(Ev::RemoveButTeam, &[teamid as i32]) {
        remove_players_but_team_now(teamid);
    }
}

pub fn remove_players_but_team_now(teamid: i16) {
    unsafe {
        for i in 0..players.len() {
            let mut player = players[i];
            if player.get_team_id() != teamid {
                player.state = PlayerState::Dead;
            }
        }
    }
}

pub fn remove_players_but_highest_scoring() {
    if !net_random::event(Ev::RemoveButHighest, &[]) {
        remove_players_but_highest_scoring_now();
    }
}

pub fn remove_players_but_highest_scoring_now() {
    unsafe {
        let mut iMaxScore: i16 = -1;

        //Figure out what the maximum score is
        for iScore in 0..score_cnt {
            if score[iScore as usize].score > iMaxScore {
                iMaxScore = score[iScore as usize].score;
            }
        }

        //Remove all players that don't have that max score
        for i in 0..players.len() {
            let mut player = players[i];
            if player.score().score < iMaxScore {
                player.state = PlayerState::Dead;
            }
        }
    }
}

pub fn setup_score_board(fOrderMatters: bool) {
    if !net_random::event(Ev::ScoreBoard, &[fOrderMatters as i32]) {
        setup_score_board_now(fOrderMatters);
    }
}

pub fn setup_score_board_now(fOrderMatters: bool) {
    unsafe {
        let mut doneWithScore: [bool; 4] = [false, false, false, false];

        let mut oldmax: i16;
        let mut max: i16 = -1;
        for i in 0..score_cnt {
            oldmax = max;
            max = -1;

            for j in 0..score_cnt {
                if !doneWithScore[j as usize] {
                    let sj = score[j as usize];
                    //The boxes minigame doesn't use "score" it uses "subscore[0]" to determine the winner
                    if game_values.gamemode.gamemode == game_mode_boxes_minigame {
                        if max == -1
                            || sj.subscore[0] > score[max as usize].subscore[0]
                            || (sj.subscore[0] == score[max as usize].subscore[0] && sj.score > score[max as usize].score)
                        {
                            //or it is tied but they died later in the game
                            max = j;
                        }
                    } else {
                        //If this player's score is bigger
                        if max == -1
                            || sj.score > score[max as usize].score
                            || (sj.score == score[max as usize].score && sj.order > score[max as usize].order)
                        {
                            //or it is tied but they died later in the game
                            max = j;
                        }
                    }
                }
            }

            score[max as usize].displayorder = i;
            score[max as usize].place = i;

            if !fOrderMatters && i > 0 {
                if game_values.gamemode.gamemode == game_mode_boxes_minigame {
                    if score[oldmax as usize].subscore[0] == score[max as usize].subscore[0]
                        && score[oldmax as usize].score == score[max as usize].score
                    {
                        score[max as usize].place = score[oldmax as usize].place;
                    }
                } else if score[oldmax as usize].score == score[max as usize].score {
                    score[max as usize].place = score[oldmax as usize].place;
                }
            }

            doneWithScore[max as usize] = true; //this is the next biggest score - it doesn't belong to the remaining scores from now on
        }

        //Add the treasure chests to the map in world mode if there were any awards for winning this match
        if game_values.matchtype == MatchType::World && game_values.gamemode.winningteam > -1 && game_values.gamemode.gamemode != game_mode_bonus {
            let tourStop = game_values.tourstops[game_values.tourstopcurrent];
            let iNumBonuses: i16 = tourStop.iNumBonuses;

            for iBonus in 0..iNumBonuses {
                if tourStop.wsbBonuses[iBonus as usize].iWinnerPlace == 0 {
                    let iBonusType: i16 = tourStop.wsbBonuses[iBonus as usize].iBonus;
                    objectcontainer[0].add(Ptr::new_box(PU_TreasureChestBonus::new(Ptr::from_mut(&mut rm.spr_bonuschest), 1, 0, 30, 30, 1, 1, iBonusType)));
                    game_values.flags.noexittimer = 0;
                    game_values.flags.noexit = false;
                }
            }
        }
    }
}

pub fn show_score_board() {
    if !net_random::event(Ev::ShowScoreBoard, &[]) {
        show_score_board_now();
    }
}

pub fn show_score_board_now() {
    unsafe {
        game_values.flags.showscoreboard = true;

        let mut iScoreboardElementHeight: i16 = 45;
        if game_values.gamemode.gamemode == game_mode_health
            || game_values.gamemode.gamemode == game_mode_collection
            || game_values.gamemode.gamemode == game_mode_boxes_minigame
        {
            iScoreboardElementHeight = 63;
        }

        for i in 0..score_cnt {
            let mut s = score[i as usize];
            s.destx = (309 - 34 * game_values.teamcounts[i as usize] as i32) as i16;
            s.desty = (s.displayorder as i32 * iScoreboardElementHeight as i32 + 140) as i16;
        }

        if game_values.music {
            rm.sfx_invinciblemusic.stop();
            rm.sfx_timewarning.stop();
            rm.sfx_slowdownmusic.stop();

            rm.backgroundmusic[1].play(true, false);
        }
    }
}

//Returns true if all but one team is dead
pub fn remove_team(teamid: i16) -> bool {
    match crate::smw::net_outcomes::remove_team_event(teamid) {
        Some(result) => result,
        None => remove_team_now(teamid),
    }
}

pub fn remove_team_now(teamid: i16) -> bool {
    unsafe {
        //If we have already removed this team then return
        if score[teamid as usize].order > -1 {
            return game_values.flags.teamdeadcounter as i32 == score_cnt as i32 - 1;
        }

        //kill all players on the dead team
        let mut iAnnouncementColor: i16 = -1;
        for i in 0..players.len() {
            let mut player = players[i];
            if player.get_team_id() == teamid {
                if iAnnouncementColor == -1 {
                    iAnnouncementColor = player.get_color_id();
                }

                player.state = PlayerState::Dead;
            }
        }

        score[teamid as usize].order = game_values.flags.teamdeadcounter;
        game_values.flags.teamdeadcounter += 1;

        //Announce that a team was removed
        if game_values.deadteamnotice && (game_values.flags.teamdeadcounter as i32) < score_cnt as i32 - 1 {
            eyecandy[2].emplace(EC_Announcement::new(
                Ptr::from_mut(&mut rm.game_font_large),
                Ptr::from_mut(&mut rm.spr_announcementicons),
                "Team Removed!".to_string(),
                iAnnouncementColor,
                90,
                200,
            ));
            rm.sfx_announcer[(iAnnouncementColor as i32 + 16) as usize].play();
        }

        game_values.flags.teamdeadcounter as i32 == score_cnt as i32 - 1
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

    pub fn displayplayertext(&mut self) {
        if self.winningteam > -1 {
            unsafe {
                for i in 0..players.len() {
                    let mut player = players[i];
                    if player.get_team_id() == self.winningteam {
                        player.spawn_text("Winner!");
                    }
                }
            }
        }
    }

    pub fn playwarningsound(&mut self) {
        if self.playedwarningsound {
            return;
        }

        self.playedwarningsound = true;
        unsafe {
            rm.sfx_invinciblemusic.stop();

            if game_values.music && game_values.sound {
                rm.backgroundmusic[0].stop();
            }

            if_sound_on_play(&mut rm.sfx_timewarning);
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

    pub fn get_highest_score_player(&mut self, fGetHighest: bool) -> Ptr<CPlayer> {
        unsafe {
            let mut count: i16 = 1;
            let mut tiedplayers: [i16; 4] = [0; 4];
            tiedplayers[0] = 0;

            //Find the first non-dead player and use them for the first player to compare to
            let mut j: usize = 0;
            while j < players.len() {
                if !players[j].isdead() {
                    count = 1;
                    tiedplayers[0] = j as i16;
                    break;
                }
                j += 1;
            }

            //Loop through all players, comparing scores to find the highest/lowest
            let mut i = j + 1;
            while i < players.len() {
                if !players[i].isdead() {
                    let si = players[i].score().score;
                    let s0 = players[tiedplayers[0] as usize].score().score;
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

            players[tiedplayers[RANDOM_INT(count as i32) as usize] as usize]
        }
    }

    //Returns number of players in list
    // Faithful to C++: the bubble sort reorders the global `players`, not `outPlayers`.
    pub fn get_score_ranked_player_list(&mut self, outPlayers: &mut [Ptr<CPlayer>; 4], fGetHighest: bool) -> i16 {
        unsafe {
            let mut iNumPlayersInList: i16 = 0;

            for i in 0..players.len() {
                let player = players[i];
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
                    let a = players[iIndex as usize].score().score;
                    let b = players[iIndex as usize + 1].score().score;
                    let swap = (fGetHighest && a < b) || (!fGetHighest && a > b) || (a == b && RANDOM_BOOL() && {
                        let r = iRandom < 5;
                        iRandom += 1;
                        r
                    });
                    if swap {
                        players.swap(iIndex as usize, iIndex as usize + 1);

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

pub fn cgamemode_init<T: CGameModeTrait + ?Sized>(this: &mut T) {
    let this = this.gm_mut();
    if this.goal == 1000 {
        this.goal = 999; //Cap goal for 3 digit scoreboard
    }

    this.winningteam = -1;
    this.gameover = false;
    this.playedwarningsound = false;

    unsafe {
        for iScore in 0..score_cnt {
            score[iScore as usize].set_score(0);
        }
    }
}

pub fn cgamemode_think<T: CGameModeTrait + ?Sized>(this: &mut T) {
    unsafe {
        if netplay.active {
            for k in 0..players.len() {
                if netplay.player_disconnected[k] {
                    players[k].spawn_text("Disconnected!");
                }
            }
        }
    }

    if this.gm().gameover {
        this.gm_mut().displayplayertext();
    }
}

pub fn cgamemode_playerkilledplayer<T: CGameModeTrait + ?Sized>(this: &mut T, mut inflictor: Ptr<CPlayer>, other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
    //Penalize killing your team mates
    if !this.gm().gameover {
        if inflictor.get_team_id() == other.get_team_id() {
            inflictor.score().adjust_score(-1);
        } else {
            inflictor.score().adjust_score(1);
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

pub fn cgamemode_playerextraguy<T: CGameModeTrait + ?Sized>(this: &mut T, mut player: Ptr<CPlayer>, iType: i16) {
    if !this.gm().gameover {
        player.score().adjust_score(iType);
    }
}
