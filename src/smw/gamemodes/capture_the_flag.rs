//! Port of src/smw/gamemodes/CaptureTheFlag.cpp

use crate::common::game_mode::*;
use crate::common::game_values::if_sound_on_play;
use crate::common::moving_object_types::movingobject_flag;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::*;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::players;
use crate::smw::objects::carriable::co_flag::CO_Flag;
use crate::smw::objects::moving::mo_flag_base::MO_FlagBase;
use crate::smw::player::CPlayer;

//Capture The Flag mode - each team has a base and a flag
//Protect your colored flag from being taken and score a point
//for stealing another teams flag and returning it to your base
pub struct CGM_CaptureTheFlag {
    pub cgame_mode: CGameMode,
}
impl_base!(CGM_CaptureTheFlag => cgame_mode: CGameMode);

impl CGM_CaptureTheFlag {
    pub fn new() -> Self {
        let mut this = CGM_CaptureTheFlag { cgame_mode: CGameMode::new() };
        this.goal = 20;
        this.gamemode = game_mode_ctf;

        this.setup_mode_strings("Capture The Flag", "Flags", 5);
        this
    }
}

impl CGameModeTrait for CGM_CaptureTheFlag {
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self) {
        cgamemode_init(self);

        let mut fTeamUsed: [bool; 4] = [false, false, false, false];

        unsafe {
            for i in 0..players.len() {
                let player = players[i];
                let iTeamID: i16 = player.get_team_id();
                if !fTeamUsed[iTeamID as usize] {
                    fTeamUsed[iTeamID as usize] = true;

                    let iColorID: i16 = player.get_color_id();
                    let base: Ptr<MO_FlagBase> = Ptr::new_box(MO_FlagBase::new(Ptr::from_mut(&mut rm.spr_flagbases), iTeamID, iColorID));
                    objectcontainer[0].add(base);

                    if !game_values.gamemodesettings.flag.centerflag {
                        let flag: Ptr<CO_Flag> = CO_Flag::new(Ptr::from_mut(&mut rm.spr_flags), base, iTeamID, iColorID);
                        objectcontainer[1].add(flag);
                    }
                }
            }

            if game_values.gamemodesettings.flag.centerflag {
                let centerflag: Ptr<CO_Flag> = CO_Flag::new(Ptr::from_mut(&mut rm.spr_flags), Ptr::null(), -1, -1);
                objectcontainer[1].add(centerflag);
            }
        }
    }

    fn playerkilledplayer(&mut self, _inflictor: Ptr<CPlayer>, _other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgamemode_playerkilledself(self, player, style);

        let mut item = player.carriedItem;
        if !item.is_null() && item.get_moving_object_type() == movingobject_flag {
            item.as_any().downcast_mut::<CO_Flag>().expect("carried flag").place_flag();
            unsafe {
                if_sound_on_play(&mut rm.sfx_transform);
            }
        }

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, mut player: Ptr<CPlayer>, iType: i16) {
        if self.gameover {
            return;
        }

        player.score().adjust_score(iType);
        self.check_winner(player);
    }

    fn check_winner(&mut self, mut player: Ptr<CPlayer>) -> PlayerKillType {
        if self.goal > -1 {
            if player.score().score >= self.goal {
                player.score().set_score(self.goal);
                self.winningteam = player.get_team_id();
                self.gameover = true;

                remove_players_but_team(self.winningteam);
                setup_score_board(false);
                show_score_board();
            } else if player.score().score as i32 >= self.goal as i32 - 2 && !self.playedwarningsound {
                self.playwarningsound();
            }
        }

        PlayerKillType::Normal
    }
}
