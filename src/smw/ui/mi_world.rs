//! Port of src/smw/ui/MI_World.cpp

use crate::common::eyecandy::{EC_Announcement, EC_SingleAnimation};
use crate::common::file_list::WorldMusicCategory;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::global_constants::{NUM_POWERUPS, TILESIZE, TWO_PI};
use crate::common::input::{COutputControl, CPlayerInput, DEVICE_KEYBOARD};
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::sfx::sfxMusic;
use crate::common::ui::menu_code::*;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use crate::smw::gs_gameplay::lookup_team_id;
use crate::smw::world::{g_worldmap, WorldMapTile, WORLD_BRIDGE_SPRITE_OFFSET};
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::trig::{cosf, sinf};
use sdl2::sys::{SDL_Rect, SDL_Surface};

pub struct MI_World {
    pub ui_control: UI_Control,

    iState: i16,
    iStateTransition: [i16; 4],
    iItemPopupDrawY: [i16; 4],
    iPopupOffsets: [i16; 4],
    iPopupOffsetsCurrent: [i16; 4],
    iPopupOrder: [i16; 4],
    iNumPopups: i16,
    iStoredItemPopupDrawY: i16,

    iPopupFlag: [bool; 4],

    iItemCol: [i16; 4],
    iItemPage: [i16; 4],

    map_sprites: [gfxSprite; 2],
    rectSrcSurface: SDL_Rect,
    rectDstSurface: SDL_Rect,

    iCurrentSurfaceIndex: i16,
    iCycleIndex: i16,
    iDrawFullRefresh: i16,

    iAnimationFrame: i16,

    iMapOffsetX: i16,
    iMapOffsetY: i16,

    iMapDrawOffsetCol: i16,
    iMapDrawOffsetRow: i16,

    iNextMapDrawOffsetCol: i16,
    iNextMapDrawOffsetRow: i16,

    iDrawWidth: i16,
    iDrawHeight: i16,
    iSrcOffsetX: i16,
    iSrcOffsetY: i16,
    iDstOffsetX: i16,
    iDstOffsetY: i16,

    iControllingTeam: i16,
    iControllingPlayerId: i16,
    iReturnDirection: i16,

    fForceStageStart: bool,
    iVehicleId: i16,

    iWarpCol: i16,
    iWarpRow: i16,

    iScreenfade: i16,
    iScreenfadeRate: i16,

    dTeleportStarRadius: f32,
    dTeleportStarAngle: f32,
    iTeleportStarAnimationFrame: i16,
    iTeleportStarAnimationTimer: i16,

    fNoInterestingMoves: bool,

    iSleepTurns: i16,
    fUsingCloud: bool,

    iPressSelectTimer: i16,
    pressSelectKeys: Ptr<COutputControl>,
}
crate::impl_base!(MI_World => ui_control: UI_Control);

/// `game_values.teamids[iTeam][iMember]` read as the flat C++ array, where a member index of 3 runs into the next row.
unsafe fn teamids_flat(iTeam: i16, iMember: i16) -> i16 {
    let idx = iTeam as usize * 3 + iMember as usize;
    if idx < 12 {
        game_values.teamids[idx / 3][idx % 3]
    } else {
        game_values.teamcounts[idx - 12]
    }
}

impl Default for MI_World {
    fn default() -> Self {
        Self::new()
    }
}

impl MI_World {
    pub fn new() -> Self {
        MI_World {
            ui_control: UI_Control::new(0, 0),
            iState: 0,
            iStateTransition: [0; 4],
            iItemPopupDrawY: [0; 4],
            iPopupOffsets: [0; 4],
            iPopupOffsetsCurrent: [0; 4],
            iPopupOrder: [0; 4],
            iNumPopups: 0,
            iStoredItemPopupDrawY: 0,
            iPopupFlag: [false; 4],
            iItemCol: [0; 4],
            iItemPage: [0; 4],
            map_sprites: [gfxSprite::blank(768, 608), gfxSprite::blank(768, 608)],
            rectSrcSurface: SDL_Rect { x: 0, y: 0, w: 768, h: 608 },
            rectDstSurface: SDL_Rect { x: 0, y: 0, w: App::screenWidth, h: App::screenHeight },
            iCurrentSurfaceIndex: 0,
            iCycleIndex: 0,
            iDrawFullRefresh: 0,
            iAnimationFrame: 0,
            iMapOffsetX: 0,
            iMapOffsetY: 0,
            iMapDrawOffsetCol: 0,
            iMapDrawOffsetRow: 0,
            iNextMapDrawOffsetCol: 0,
            iNextMapDrawOffsetRow: 0,
            iDrawWidth: 0,
            iDrawHeight: 0,
            iSrcOffsetX: 0,
            iSrcOffsetY: 0,
            iDstOffsetX: 0,
            iDstOffsetY: 0,
            iControllingTeam: 0,
            iControllingPlayerId: 0,
            iReturnDirection: 0,
            fForceStageStart: false,
            iVehicleId: 0,
            iWarpCol: 0,
            iWarpRow: 0,
            iScreenfade: 0,
            iScreenfadeRate: 0,
            dTeleportStarRadius: 0.0,
            dTeleportStarAngle: 0.0,
            iTeleportStarAnimationFrame: 0,
            iTeleportStarAnimationTimer: 0,
            fNoInterestingMoves: false,
            iSleepTurns: 0,
            fUsingCloud: false,
            iPressSelectTimer: 0,
            pressSelectKeys: Ptr::null(),
        }
    }

    /// `iStateTransition[iTeamId]`; `LookupTeamID` gives -1 for players outside the game, and the C++
    /// then reads the preceding member `iState`.
    fn state_transition(&self, iTeamId: i16) -> i16 {
        if iTeamId < 0 {
            self.iState
        } else {
            self.iStateTransition[iTeamId as usize]
        }
    }

    pub fn init(&mut self) {
        unsafe {
            self.iCycleIndex = 0;
            g_worldmap.reset_draw_cycle();

            self.iAnimationFrame = 0;
            self.iDrawFullRefresh = 0;

            self.iMapDrawOffsetCol = 0;
            self.iMapDrawOffsetRow = 0;

            self.iNextMapDrawOffsetCol = 0;
            self.iNextMapDrawOffsetRow = 0;

            self.iState = -2;

            for iTeam in 0..4 {
                self.iStateTransition[iTeam] = 0;
                self.iItemPopupDrawY[iTeam] = 0;
                self.iPopupFlag[iTeam] = false;
            }

            self.iNumPopups = 0;

            self.iStoredItemPopupDrawY = -48;

            self.iVehicleId = -1;

            self.set_map_offset();
            self.reposition_map_image();

            g_worldmap.draw_map_to_surface(-1, true, self.map_sprites[0].get_surface(), self.iMapDrawOffsetCol, self.iMapDrawOffsetRow, self.iAnimationFrame);
            g_worldmap.draw_map_to_surface(-1, true, self.map_sprites[1].get_surface(), self.iMapDrawOffsetCol, self.iMapDrawOffsetRow, self.iAnimationFrame);

            self.dTeleportStarRadius = 300.0f32;
            self.dTeleportStarAngle = 0.0f32;
            self.iTeleportStarAnimationFrame = 0;
            self.iTeleportStarAnimationTimer = 0;

            self.fForceStageStart = false;
            self.fNoInterestingMoves = false;
            self.iSleepTurns = 0;
            self.fUsingCloud = false;
            game_values.worldpointsbonus = -1;

            self.iPressSelectTimer = 0;
            self.pressSelectKeys = Ptr::null();

            self.iDrawWidth = if g_worldmap.iWidth < 20 { g_worldmap.iWidth << 5 } else { App::screenWidth as i16 };
            self.iDrawHeight = if g_worldmap.iHeight < 15 { g_worldmap.iHeight << 5 } else { App::screenHeight as i16 };

            self.iSrcOffsetX = 0;
            self.iSrcOffsetY = 0;
            self.iDstOffsetX = 0;
            self.iDstOffsetY = 0;

            if g_worldmap.iWidth < 20 {
                self.iDstOffsetX = (20 - g_worldmap.iWidth) << 4;
            }

            if g_worldmap.iHeight < 15 {
                self.iDstOffsetY = (15 - g_worldmap.iHeight) << 4;
            }
        }
    }

    /// `MI_World::SetControllingTeam`; hides `UI_Control::SetControllingTeam` like the C++.
    pub fn set_controlling_team(&mut self, iWinningTeam: i16) {
        unsafe {
            self.iControllingTeam = iWinningTeam;
            let ct = self.iControllingTeam as usize;
            let r = RANDOM_INT(game_values.teamcounts[ct] as i32);
            self.iControllingPlayerId = game_values.teamids[ct][r as usize];
            g_worldmap.set_player_sprite(self.iControllingPlayerId);

            self.fNoInterestingMoves = false;
        }
    }

    pub fn display_team_control_announcement(&mut self) {
        unsafe {
            let ct = self.iControllingTeam as usize;
            let szMessage = if game_values.teamcounts[ct] <= 1 {
                format!("Player {} Is In Control", game_values.teamids[ct][0] as i32 + 1)
            } else {
                format!("Team {} Is In Control", self.iControllingTeam as i32 + 1)
            };

            let mut parent = self.m_parentMenu;
            parent.eyeCandy.emplace(EC_Announcement::new(
                Ptr::from_mut(&mut rm.menu_font_large),
                Ptr::from_mut(&mut rm.spr_announcementicons),
                szMessage,
                game_values.colorids[self.iControllingPlayerId as usize],
                120,
                100,
            ));
        }
    }

    pub fn set_current_stage_to_completed(&mut self, iWinningTeam: i16) {
        unsafe {
            let mut iBonusAdd: i16 = 0;
            let mut iBonusMult: i16 = 1;

            if game_values.worldpointsbonus == 0 {
                iBonusMult = 0;
            } else if game_values.worldpointsbonus >= 1 && game_values.worldpointsbonus <= 3 {
                iBonusAdd = game_values.worldpointsbonus;
            } else if game_values.worldpointsbonus >= 4 && game_values.worldpointsbonus <= 5 {
                iBonusMult = game_values.worldpointsbonus - 2;
            }

            let wt = iWinningTeam as usize;
            if self.iVehicleId >= 0 {
                let add = g_worldmap.get_vehicle_stage_score(self.iVehicleId) as i32 * iBonusMult as i32 + iBonusAdd as i32;
                game_values.tournament_scores[wt].total = (game_values.tournament_scores[wt].total as i32 + add) as i16;
                g_worldmap.remove_vehicle(self.iVehicleId);
            } else {
                let iPlayerCurrentTile = g_worldmap.get_player_current_tile();

                let tile = g_worldmap.tiles.at_mut(iPlayerCurrentTile.x as usize, iPlayerCurrentTile.y as usize);
                //tile->iForegroundSprite = game_values.colorids[game_values.teamids[iWinningTeam][0]] + WORLD_WINNING_TEAM_SPRITE_OFFSET; //Update with team completed sprite
                //tile->fAnimated = false; //Update with team completed sprite
                tile.iCompleted = game_values.colorids[game_values.teamids[wt][0] as usize];
                let iType = tile.iType;

                self.update_tile_both(iPlayerCurrentTile.x - self.iMapDrawOffsetCol, iPlayerCurrentTile.y - self.iMapDrawOffsetRow);

                let add = game_values.tourstops[(iType - 6) as usize].iPoints as i32 * iBonusMult as i32 + iBonusAdd as i32;
                game_values.tournament_scores[wt].total = (game_values.tournament_scores[wt].total as i32 + add) as i16;
            }

            //Only advance the turn if the players played a real game, no bonus houses
            if game_values.singleplayermode == -1 {
                self.advance_turn();
            }
        }
    }

    pub fn clear_cloud(&mut self) {
        self.fUsingCloud = false;
    }

    fn advance_turn(&mut self) {
        unsafe {
            if self.iSleepTurns > 0 {
                self.iSleepTurns -= 1;
                if self.iSleepTurns <= 0 {
                    rm.backgroundmusic[5] = sfxMusic::from_file(worldmusiclist.current_music(g_worldmap.get_music_category(), g_worldmap.get_world_name()))
                        .unwrap_or_else(|e| std::panic::panic_any(e));
                    rm.backgroundmusic[5].play(false, false);
                }
            } else {
                g_worldmap.move_vehicles();
            }

            g_worldmap.move_bridges();

            //Update the map with the flipped bridges
            //TODO:: Need to update just bridge tiles across both surfaces

            self.fNoInterestingMoves = false;
        }
    }

    fn update_map_surface(&mut self, iCycleIndex: i16) {
        if iCycleIndex >= 0 && iCycleIndex <= 15 {
            unsafe {
                g_worldmap.draw_map_to_surface(
                    iCycleIndex,
                    self.iDrawFullRefresh > 0,
                    self.map_sprites[(1 - self.iCurrentSurfaceIndex) as usize].get_surface(),
                    self.iNextMapDrawOffsetCol,
                    self.iNextMapDrawOffsetRow,
                    self.iAnimationFrame,
                );
            }
        }
    }

    fn set_map_offset(&mut self) {
        unsafe {
            let iPlayerDrawPos = g_worldmap.get_player_position();
            let w32 = (g_worldmap.iWidth as i32) << 5;
            let h32 = (g_worldmap.iHeight as i32) << 5;
            let px = iPlayerDrawPos.x as i32;
            let py = iPlayerDrawPos.y as i32;

            if g_worldmap.iWidth > 20 {
                if px < w32 - 336 && px > 304 {
                    self.iMapOffsetX = (304 - px) as i16;
                } else if px <= 304 {
                    self.iMapOffsetX = 0;
                } else {
                    self.iMapOffsetX = (App::screenWidth - w32) as i16;
                }
            } else {
                self.iMapOffsetX = ((App::screenWidth - w32) >> 1) as i16;
            }

            if g_worldmap.iHeight > 15 {
                if py < h32 - 256 && py > 224 {
                    self.iMapOffsetY = (224 - py) as i16;
                } else if py <= 224 {
                    self.iMapOffsetY = 0;
                } else {
                    self.iMapOffsetY = (App::screenHeight - h32) as i16;
                }
            } else {
                self.iMapOffsetY = ((App::screenHeight - h32) >> 1) as i16;
            }
        }
    }

    fn reposition_map_image(&mut self) {
        unsafe {
            let iPlayerCurrentTile = g_worldmap.get_player_current_tile();

            if g_worldmap.iWidth > 24 {
                self.iMapDrawOffsetCol = iPlayerCurrentTile.x - 11;
                if self.iMapDrawOffsetCol < 0 {
                    self.iMapDrawOffsetCol = 0;
                } else if self.iMapDrawOffsetCol > g_worldmap.iWidth - 24 {
                    self.iMapDrawOffsetCol = g_worldmap.iWidth - 24;
                }
            }

            if g_worldmap.iHeight > 19 {
                self.iMapDrawOffsetRow = iPlayerCurrentTile.y - 9;
                if self.iMapDrawOffsetRow < 0 {
                    self.iMapDrawOffsetRow = 0;
                } else if self.iMapDrawOffsetRow > g_worldmap.iHeight - 19 {
                    self.iMapDrawOffsetRow = g_worldmap.iHeight - 19;
                }
            }

            self.iNextMapDrawOffsetCol = self.iMapDrawOffsetCol;
            self.iNextMapDrawOffsetRow = self.iMapDrawOffsetRow;
        }
    }

    fn restart_draw_cycle_if_needed(&mut self) {
        unsafe {
            self.iNextMapDrawOffsetCol = self.iMapDrawOffsetCol;
            self.iNextMapDrawOffsetRow = self.iMapDrawOffsetRow;

            let iPlayerDestTile = g_worldmap.get_player_dest_tile();

            if g_worldmap.iWidth > 24 {
                self.iNextMapDrawOffsetCol = iPlayerDestTile.x - 11;
                if self.iNextMapDrawOffsetCol < 0 {
                    self.iNextMapDrawOffsetCol = 0;
                } else if self.iNextMapDrawOffsetCol > g_worldmap.iWidth - 24 {
                    self.iNextMapDrawOffsetCol = g_worldmap.iWidth - 24;
                }
            }

            if g_worldmap.iHeight > 19 {
                self.iNextMapDrawOffsetRow = iPlayerDestTile.y - 9;
                if self.iNextMapDrawOffsetRow < 0 {
                    self.iNextMapDrawOffsetRow = 0;
                } else if self.iNextMapDrawOffsetRow > g_worldmap.iHeight - 19 {
                    self.iNextMapDrawOffsetRow = g_worldmap.iHeight - 19;
                }
            }

            //Reset the draw cycle if the map offset col/row is going to move
            if self.iMapDrawOffsetCol != self.iNextMapDrawOffsetCol || self.iMapDrawOffsetRow != self.iNextMapDrawOffsetRow {
                self.iCycleIndex = 0;
                g_worldmap.reset_draw_cycle();
                self.iDrawFullRefresh = 1;
            }
        }
    }

    /// The paired `g_worldmap.UpdateTile(map_sprites[0..2], ...)` calls.
    fn update_tile_both(&mut self, col: i16, row: i16) {
        unsafe {
            g_worldmap.update_tile(self.map_sprites[0].get_surface(), col, row, self.iMapDrawOffsetCol, self.iMapDrawOffsetRow, self.iAnimationFrame);
            g_worldmap.update_tile(self.map_sprites[1].get_surface(), col, row, self.iMapDrawOffsetCol, self.iMapDrawOffsetRow, self.iAnimationFrame);
        }
    }

    fn use_powerup(&mut self, iPlayer: i16, iTeam: i16, iIndex: i16, fPopupIsUp: bool) -> bool {
        unsafe {
            let iPlayerCurrentTile = g_worldmap.get_player_current_tile();

            let t = iTeam as usize;
            let iPowerup = game_values.worldpowerups[t][iIndex as usize];
            let p = iPowerup as i32;
            let mut fUsedItem = false;

            if p < NUM_POWERUPS {
                /*
                //Comment this in to give the powerup to all members of the team
                for (short iPlayer = 0; iPlayer < game_values.teamcounts[iTeam]; iPlayer++)
                {
                    game_values.storedpowerups[game_values.teamids[iTeam][iPlayer]] = iPowerup;
                }*/

                //Give the powerup only to the player that selected it
                game_values.storedpowerups[iPlayer as usize] = iPowerup;

                if_sound_on_play(&mut rm.sfx_collectpowerup);
                fUsedItem = true;
            } else if p == NUM_POWERUPS {
                //Music Box (put vehicles to sleep)
                self.iSleepTurns = (RANDOM_INT(4) + 2) as i16;
                fUsedItem = true;
                if_sound_on_play(&mut rm.sfx_collectpowerup);

                rm.backgroundmusic[5] = sfxMusic::from_file(worldmusiclist.current_music(WorldMusicCategory::Sleep, "")).unwrap_or_else(|e| std::panic::panic_any(e));
                rm.backgroundmusic[5].play(false, false);
            } else if p == NUM_POWERUPS + 1 {
                //Cloud (allows player to skip stages)
                if !self.fUsingCloud && self.iState == -1 {
                    self.use_cloud(true);
                    fUsedItem = true;
                }
            } else if p == NUM_POWERUPS + 2 {
                //Player Switch
                if self.iControllingTeam != iTeam && self.iState == -1 {
                    self.set_controlling_team(iTeam);
                    self.display_team_control_announcement();

                    fUsedItem = true;
                    if_sound_on_play(&mut rm.sfx_switchpress);
                    self.iState = 6;
                    self.fNoInterestingMoves = false;
                }
            } else if p == NUM_POWERUPS + 3 && self.iState == -1 {
                //Advance Turn
                let iDest = g_worldmap.get_player_dest_tile();
                let iSprite = g_worldmap.tiles.at(iDest.x as usize, iDest.y as usize).iForegroundSprite;

                if iSprite < WORLD_BRIDGE_SPRITE_OFFSET || iSprite > WORLD_BRIDGE_SPRITE_OFFSET + 3 {
                    self.advance_turn();
                    fUsedItem = true;
                    if_sound_on_play(&mut rm.sfx_switchpress);
                }
            } else if p == NUM_POWERUPS + 4 && self.iState == -1 {
                //Revive stage
                let iDest = g_worldmap.get_player_dest_tile();
                let tile = g_worldmap.tiles.at_mut(iDest.x as usize, iDest.y as usize);

                if tile.iType >= 6 && tile.iCompleted >= 0 {
                    tile.iCompleted = -2;

                    self.update_tile_both(iDest.x - self.iMapDrawOffsetCol, iDest.y - self.iMapDrawOffsetRow);

                    let mut parent = self.m_parentMenu;
                    parent.eyeCandy.emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_poof),
                        (((iDest.x as i32) << 5) + self.iMapOffsetX as i32 - 8) as i16,
                        (((iDest.y as i32) << 5) + self.iMapOffsetY as i32 - 8) as i16,
                        4,
                        5,
                    ));

                    fUsedItem = true;
                    if_sound_on_play(&mut rm.sfx_transform);
                }
            } else if p >= NUM_POWERUPS + 5 && p <= NUM_POWERUPS + 8 && self.iState == -1 {
                //Door Keys
                let iDoorsOpened = g_worldmap.use_key((p - NUM_POWERUPS - 5) as i16, iPlayerCurrentTile.x, iPlayerCurrentTile.y, self.fUsingCloud);

                if iDoorsOpened > 0 {
                    if_sound_on_play(&mut rm.sfx_transform);

                    let iPlayerX = (((iPlayerCurrentTile.x as i32) << 5) + self.iMapOffsetX as i32) as i16;
                    let iPlayerY = (((iPlayerCurrentTile.y as i32) << 5) + self.iMapOffsetY as i32) as i16;
                    let col = iPlayerCurrentTile.x - self.iMapDrawOffsetCol;
                    let row = iPlayerCurrentTile.y - self.iMapDrawOffsetRow;
                    let ts = TILESIZE as i16;

                    if iDoorsOpened & 0x1 != 0 {
                        let mut parent = self.m_parentMenu;
                        parent.eyeCandy.emplace(EC_SingleAnimation::new(Ptr::from_mut(&mut rm.spr_fireballexplosion), iPlayerX - ts, iPlayerY, 3, 8));

                        self.update_tile_both(col - 1, row);
                    }

                    if iDoorsOpened & 0x2 != 0 {
                        let mut parent = self.m_parentMenu;
                        parent.eyeCandy.emplace(EC_SingleAnimation::new(Ptr::from_mut(&mut rm.spr_fireballexplosion), iPlayerX + ts, iPlayerY, 3, 8));

                        self.update_tile_both(col + 1, row);
                    }

                    if iDoorsOpened & 0x4 != 0 {
                        let mut parent = self.m_parentMenu;
                        parent.eyeCandy.emplace(EC_SingleAnimation::new(Ptr::from_mut(&mut rm.spr_fireballexplosion), iPlayerX, iPlayerY - ts, 3, 8));

                        self.update_tile_both(col, row - 1);
                    }

                    if iDoorsOpened & 0x8 != 0 {
                        let mut parent = self.m_parentMenu;
                        parent.eyeCandy.emplace(EC_SingleAnimation::new(Ptr::from_mut(&mut rm.spr_fireballexplosion), iPlayerX, iPlayerY + ts, 3, 8));

                        self.update_tile_both(col, row + 1);
                    }

                    fUsedItem = true;
                    self.fNoInterestingMoves = false;
                }
            } else if p >= NUM_POWERUPS + 9 && p <= NUM_POWERUPS + 14 {
                //Stage Points Modifiers
                game_values.worldpointsbonus = (p - NUM_POWERUPS - 9) as i16;
                fUsedItem = true;
                if_sound_on_play(&mut rm.sfx_switchpress);
            }

            if fUsedItem {
                game_values.worldpowerupcount[t] -= 1;
                let iNumItems = game_values.worldpowerupcount[t];

                let mut iItem = iIndex;
                while iItem < iNumItems {
                    game_values.worldpowerups[t][iItem as usize] = game_values.worldpowerups[t][iItem as usize + 1];
                    iItem += 1;
                }

                if fPopupIsUp {
                    self.iStateTransition[t] = 2;
                }
            } else {
                if_sound_on_play(&mut rm.sfx_stun);
            }

            fUsedItem
        }
    }

    fn init_game(&mut self, iStage: i16, iPlayer: i16, fNeedAiControl: bool) -> MenuCodeEnum {
        unsafe {
            game_values.tourstopcurrent = iStage as usize;
            game_values.worldskipscoreboard = false;

            let fBonusHouse = game_values.tourstops[game_values.tourstopcurrent].iStageType == 1;

            if fBonusHouse {
                game_values.singleplayermode = iPlayer;
            }

            for iTeam in 0..4 {
                self.iPopupFlag[iTeam] = false;
            }

            if fNeedAiControl || fBonusHouse {
                MENU_CODE_TOUR_STOP_CONTINUE_FORCED
            } else {
                MENU_CODE_WORLD_STAGE_START
            }
        }
    }

    fn use_cloud(&mut self, fUseCloud: bool) {
        unsafe {
            self.fUsingCloud = fUseCloud;
            if_sound_on_play(&mut rm.sfx_transform);

            let iPlayerDrawPos = g_worldmap.get_player_position();
            let mut parent = self.m_parentMenu;
            parent.eyeCandy.emplace(EC_SingleAnimation::new(
                Ptr::from_mut(&mut rm.spr_poof),
                (iPlayerDrawPos.x as i32 + self.iMapOffsetX as i32 - 8) as i16,
                (iPlayerDrawPos.y as i32 + self.iMapOffsetY as i32 - 8) as i16,
                4,
                5,
            ));
        }
    }

    /// The movement test shared by the four directions of `SendInput`.
    #[allow(clippy::too_many_arguments)]
    unsafe fn can_move(&self, tile: &WorldMapTile, iDirection: usize, iBridgeType: i16, iDoorCol: i16, iDoorRow: i16, iReturnDir: i16, fVehicleInTile: bool) -> bool {
        (tile.fConnection[iDirection] || tile.iConnectionType == iBridgeType)
            && !g_worldmap.is_door(iDoorCol, iDoorRow)
            && (((tile.iCompleted >= -1 || (tile.iType >= 6 && game_values.tourstops[(tile.iType - 6) as usize].iStageType == 1)) && (!fVehicleInTile || self.iSleepTurns > 0))
                || self.iReturnDirection == iReturnDir
                || self.fUsingCloud)
    }
}

impl UI_ControlTrait for MI_World {
    crate::impl_ctl!();

    fn update(&mut self) {
        unsafe {
            let mut fPlayerVehicleCollision = false;
            let fPlayerMoveDone = g_worldmap.update(&mut fPlayerVehicleCollision);

            let iPlayerDrawPos = g_worldmap.get_player_position();
            let iPlayerState = g_worldmap.get_player_state();

            self.update_map_surface(self.iCycleIndex);

            self.iCycleIndex += 1;
            if self.iCycleIndex > 16 {
                self.iCurrentSurfaceIndex = 1 - self.iCurrentSurfaceIndex;

                if self.iDrawFullRefresh == 1 {
                    self.iDrawFullRefresh = 2;
                } else {
                    self.iDrawFullRefresh = 0;
                }

                self.iCycleIndex = 0;
                self.iAnimationFrame += TILESIZE as i16;

                if self.iAnimationFrame >= 128 {
                    self.iAnimationFrame = 0;
                }
            }

            if fPlayerMoveDone {
                self.reposition_map_image();
            }

            if self.iSleepTurns <= 0 && !self.fUsingCloud && (fPlayerVehicleCollision || fPlayerMoveDone) {
                let iStage = g_worldmap.get_vehicle_in_player_tile(&mut self.iVehicleId);
                if iStage >= 0 && iStage < g_worldmap.iNumStages {
                    game_values.tourstopcurrent = iStage as usize;
                    self.fForceStageStart = true;
                }
            }

            let w32 = (g_worldmap.iWidth as i32) << 5;
            let h32 = (g_worldmap.iHeight as i32) << 5;

            //Player is moving from one tile to the next (up)
            if iPlayerState == 1 {
                if g_worldmap.iHeight > 15 && self.iMapOffsetY < 0 && (iPlayerDrawPos.y as f32) < h32 as f32 - (App::screenHeight as f32 * 0.53f32) {
                    self.iMapOffsetY += 2;
                }
            } else if iPlayerState == 2 {
                //down
                if g_worldmap.iHeight > 15 && (self.iMapOffsetY as i32) > App::screenHeight - h32 && (iPlayerDrawPos.y as f32) > (App::screenHeight as f32 * 0.47f32) {
                    self.iMapOffsetY -= 2;
                }
            } else if iPlayerState == 3 {
                //left
                if g_worldmap.iWidth > 20 && self.iMapOffsetX < 0 && (iPlayerDrawPos.x as f32) < w32 as f32 - (App::screenWidth as f32 * 0.525f32) {
                    self.iMapOffsetX += 2;
                }
            } else if iPlayerState == 4 {
                //right
                if g_worldmap.iWidth > 20 && (self.iMapOffsetX as i32) > App::screenWidth - w32 && (iPlayerDrawPos.x as f32) > (App::screenWidth as f32 * 0.475f32) {
                    self.iMapOffsetX -= 2;
                }
            }

            if self.iState == -2 || self.iState >= 4 {
                self.dTeleportStarRadius += if self.iState == 4 || self.iState == 6 { 5.0f32 } else { -5.0f32 };
                self.dTeleportStarAngle -= 0.15f32;

                self.iTeleportStarAnimationTimer += 1;
                if self.iTeleportStarAnimationTimer >= 3 {
                    self.iTeleportStarAnimationTimer = 0;
                    self.iTeleportStarAnimationFrame += 32;

                    if self.iTeleportStarAnimationFrame > 32 {
                        self.iTeleportStarAnimationFrame = 0;
                    }
                }

                if self.dTeleportStarRadius <= 0.0f32 {
                    if self.iState == -2 || self.iState == 7 {
                        self.iState = -1;
                    }

                    self.dTeleportStarRadius = 0.0f32;
                }
            }

            if self.iNumPopups > 0 {
                for iTeam in 0..4i16 {
                    let t = iTeam as usize;
                    //Do inventory popup open effect
                    if self.iStateTransition[t] == 1 {
                        self.iItemPopupDrawY[t] += 4;

                        if self.iItemPopupDrawY[t] >= 32 {
                            self.iItemPopupDrawY[t] = 32;
                            self.iStateTransition[t] = 3;
                        }
                    } else if self.iStateTransition[t] == 2 {
                        // Do close effect
                        self.iItemPopupDrawY[t] -= 4;

                        if self.iItemPopupDrawY[t] <= 0 {
                            self.iItemPopupDrawY[t] = 0;
                            self.iStateTransition[t] = 0;

                            //Shift down popup menus if one was removed that was below
                            let mut fStartShift = false;
                            let mut iMoveTeam: i16 = 0;
                            while iMoveTeam < self.iNumPopups - 1 {
                                let m = iMoveTeam as usize;
                                if self.iPopupOrder[m] == iTeam {
                                    fStartShift = true;
                                }

                                if fStartShift {
                                    self.iPopupOrder[m] = self.iPopupOrder[m + 1];
                                    self.iPopupOffsets[self.iPopupOrder[m] as usize] = iMoveTeam << 6;
                                }
                                iMoveTeam += 1;
                            }

                            self.iNumPopups -= 1;
                        }
                    }
                }

                for t in 0..4usize {
                    //Transition the inventory popup to it's new location if it needs to move
                    if self.iPopupOffsetsCurrent[t] > self.iPopupOffsets[t] {
                        self.iPopupOffsetsCurrent[t] -= 4;
                        if self.iPopupOffsetsCurrent[t] < self.iPopupOffsets[t] {
                            self.iPopupOffsetsCurrent[t] = self.iPopupOffsets[t];
                        }
                    }
                }
            }

            let mut iShowStoredItems = false;
            for t in 0..4usize {
                iShowStoredItems = self.iStateTransition[t] == 1 || self.iStateTransition[t] == 3;
                if iShowStoredItems {
                    break;
                }
            }

            if iShowStoredItems && self.iStoredItemPopupDrawY < 16 {
                self.iStoredItemPopupDrawY += 8;
            } else if !iShowStoredItems && self.iStoredItemPopupDrawY > -48 {
                self.iStoredItemPopupDrawY -= 8;
            }

            if self.iState == 4 || self.iState == 5 {
                self.iScreenfade += self.iScreenfadeRate;

                if self.iState == 4 && self.iScreenfade > 255 {
                    g_worldmap.set_player_position(self.iWarpCol, self.iWarpRow);
                    self.set_map_offset();
                    self.reposition_map_image();

                    //These 3 lines allow us to only refresh the entire map to one surface and sets it up so that the
                    //other surface will update with the normal flow
                    self.iCurrentSurfaceIndex = 0;
                    self.iCycleIndex = 0;
                    g_worldmap.reset_draw_cycle();

                    self.iDrawFullRefresh = 2; //Draw one full refresh to next surface

                    g_worldmap.draw_map_to_surface(-1, true, self.map_sprites[0].get_surface(), self.iMapDrawOffsetCol, self.iMapDrawOffsetRow, self.iAnimationFrame);
                    //g_worldmap.DrawMapToSurface(-1, true, map_sprites[1].getSurface(), iMapDrawOffsetCol, iMapDrawOffsetRow, iAnimationFrame);

                    self.iState = 5;
                    self.iScreenfade = 255;
                    self.iScreenfadeRate = -8;
                } else if self.iState == 5 && self.iScreenfade < 0 {
                    self.iState = -1;
                    self.iScreenfade = 0;
                }
            } else if self.iState == 6 {
                if self.dTeleportStarRadius > 150.0f32 {
                    self.iState = 7;
                }
            }
        }
    }

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        unsafe {
            let iPlayerDrawPos = g_worldmap.get_player_position();

            if g_worldmap.iWidth > 20 {
                self.iSrcOffsetX = -self.iMapOffsetX - (self.iMapDrawOffsetCol << 5);
            }

            if g_worldmap.iHeight > 15 {
                self.iSrcOffsetY = -self.iMapOffsetY - (self.iMapDrawOffsetRow << 5);
            }

            self.rectSrcSurface = SDL_Rect { x: self.iSrcOffsetX as i32, y: self.iSrcOffsetY as i32, w: self.iDrawWidth as i32, h: self.iDrawHeight as i32 };
            self.rectDstSurface = SDL_Rect { x: self.iDstOffsetX as i32, y: self.iDstOffsetY as i32, w: self.iDrawWidth as i32, h: self.iDrawHeight as i32 };

            self.map_sprites[self.iCurrentSurfaceIndex as usize].draw_src_to(&self.rectSrcSurface, blitdest, &self.rectDstSurface);

            //Draw the world, vehicles and player
            g_worldmap.draw(self.iMapOffsetX, self.iMapOffsetY, self.iState == -1 && !self.fUsingCloud, self.iSleepTurns > 0);

            let px = iPlayerDrawPos.x as i32 + self.iMapOffsetX as i32;
            let py = iPlayerDrawPos.y as i32 + self.iMapOffsetY as i32;

            //Draw the cloud if the player is one
            if self.fUsingCloud && self.iState == -1 {
                rm.spr_worlditems.draw_src(px, py, &SDL_Rect { x: 32, y: 0, w: 32, h: 32 });
            }

            //If a points modifier is in place, display it
            if game_values.worldpointsbonus >= 0 {
                rm.spr_worlditems.draw_src(603, 5, &SDL_Rect { x: (game_values.worldpointsbonus as i32 + 9) << 5, y: 0, w: 32, h: 32 });
            }

            //Draw the teleport/warp stars effect
            if self.iState == -2 || self.iState >= 4 {
                for iStar in 0..10i16 {
                    let dAngle: f32 = self.dTeleportStarAngle + (TWO_PI / 10.0f32) * iStar as f32;
                    let iStarX = (self.dTeleportStarRadius * cosf(dAngle)) as i16;
                    let iStarY = (self.dTeleportStarRadius * sinf(dAngle)) as i16;

                    rm.spr_teleportstar.draw_src(iStarX as i32 + px, iStarY as i32 + py, &SDL_Rect { x: self.iTeleportStarAnimationFrame as i32, y: 0, w: 32, h: 32 });
                }
            }

            //If the item selector for a player is displayed
            if self.iNumPopups > 0 {
                //Draw Spots and Stored Powerups
                let mut iPlayerCount: i16 = 0;
                for iCountPlayers in 0..4usize {
                    iPlayerCount += game_values.teamcounts[iCountPlayers];
                }

                let mut iStoredPowerupBoxX: i16 = (296 - 48 * (iPlayerCount as i32 - 1)) as i16;
                for iTeamStore in 0..4usize {
                    for iMemberStore in 0..game_values.teamcounts[iTeamStore] {
                        let iPlayerId = game_values.teamids[iTeamStore][iMemberStore as usize] as usize;

                        rm.spr_worlditempopup.draw_src(
                            iStoredPowerupBoxX as i32,
                            self.iStoredItemPopupDrawY as i32,
                            &SDL_Rect { x: game_values.colorids[iPlayerId] as i32 * 48, y: 256, w: 48, h: 48 },
                        );

                        rm.spr_storedpoweruplarge.draw_src(
                            iStoredPowerupBoxX as i32 + 8,
                            self.iStoredItemPopupDrawY as i32 + 8,
                            &SDL_Rect { x: (game_values.storedpowerups[iPlayerId] as i32) << 5, y: 0, w: 32, h: 32 },
                        );

                        iStoredPowerupBoxX += 96;
                    }
                }

                for t in 0..4usize {
                    if self.iStateTransition[t] != 0 {
                        let iColorId = game_values.colorids[game_values.teamids[t][0] as usize] as i32;
                        let drawY = self.iItemPopupDrawY[t] as i32;
                        let offCur = self.iPopupOffsetsCurrent[t] as i32;
                        rm.spr_worlditempopup.draw_src(0, 448 - drawY - offCur, &SDL_Rect { x: 0, y: iColorId * 64 + 32 - drawY, w: 320, h: drawY << 1 });
                        rm.spr_worlditempopup.draw_src(320, 448 - drawY - offCur, &SDL_Rect { x: 192, y: iColorId * 64 + 32 - drawY, w: 320, h: drawY << 1 });

                        if self.iStateTransition[t] == 3 {
                            let iNumPowerups = game_values.worldpowerupcount[t];

                            if iNumPowerups > 0 {
                                rm.spr_worlditempopup.draw_src(self.iItemCol[t] as i32 * 52 + 114, 424 - offCur, &SDL_Rect { x: iColorId * 48, y: 256, w: 48, h: 48 });
                            }

                            let iStartItem: i16 = self.iItemPage[t] << 3;
                            let mut iItem = iStartItem;
                            while iItem < iStartItem + 8 && iItem < iNumPowerups {
                                let iPowerup = game_values.worldpowerups[t][iItem as usize] as i32;
                                let x = (iItem as i32 - self.iItemPage[t] as i32 * 8) * 52 + 122;
                                if iPowerup >= NUM_POWERUPS {
                                    rm.spr_worlditems.draw_src(x, 432 - offCur, &SDL_Rect { x: (iPowerup - NUM_POWERUPS) << 5, y: 0, w: 32, h: 32 });
                                } else {
                                    rm.spr_storedpoweruplarge.draw_src(x, 432 - offCur, &SDL_Rect { x: iPowerup << 5, y: 0, w: 32, h: 32 });
                                }
                                iItem += 1;
                            }
                        }
                    }
                }
            }

            if self.iState == 4 || self.iState == 5 {
                rm.menu_shade.setalpha(self.iScreenfade as u8);
                rm.menu_shade.draw(0, 0);
            }
        }
    }

    //TODO:: need a way for AI to use world items like keys
    //Also, there is a problem if you put a key into a stage but someone exits before it is collected
    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        unsafe {
            //Don't allow any input while vehicles are moving
            if g_worldmap.is_vehicle_moving() {
                return MENU_CODE_NONE;
            }

            //If the player and a vehicle collided, force the start of that level
            if self.fForceStageStart {
                self.iState = -1;

                for iTeam in 0..4 {
                    self.iNumPopups = 0;
                    self.iStateTransition[iTeam] = 0;
                    self.iItemPopupDrawY[iTeam] = 0;
                    self.iPopupFlag[iTeam] = false;
                }

                self.fForceStageStart = false;
                return MENU_CODE_TOUR_STOP_CONTINUE_FORCED;
            }

            let mut iPlayerState = g_worldmap.get_player_state();

            //Handle AI movement
            let mut fNeedAiControl = false;
            if self.iState == -1 && iPlayerState == 0 && self.iNumPopups == 0 && !self.fNoInterestingMoves {
                fNeedAiControl = true;
                let ct = self.iControllingTeam as usize;
                for iTeamMember in 0..game_values.teamcounts[ct] {
                    if game_values.playercontrol[game_values.teamids[ct][iTeamMember as usize] as usize] == 1 {
                        fNeedAiControl = false;
                        break;
                    }
                }

                if fNeedAiControl {
                    if self.iPressSelectTimer == 0 {
                        let iPlayerCurrentTile = g_worldmap.get_player_current_tile();
                        let iNextMove = g_worldmap.get_next_interesting_move(iPlayerCurrentTile.x, iPlayerCurrentTile.y);

                        //Clear out all input from cpu controlled team
                        let mut playerKeys: Ptr<COutputControl>;
                        let mut iTeamMember: i16 = 0;
                        while iTeamMember < game_values.teamcounts[ct] {
                            playerKeys = Ptr::from_mut(&mut game_values.playerInput.outputControls[game_values.teamids[ct][iTeamMember as usize] as usize]);

                            playerKeys.menu_up_mut().fPressed = false;
                            playerKeys.menu_down_mut().fPressed = false;
                            playerKeys.menu_left_mut().fPressed = false;
                            playerKeys.menu_right_mut().fPressed = false;
                            playerKeys.menu_select_mut().fPressed = false;
                            playerKeys.menu_random_mut().fPressed = false;

                            if self.iControllingTeam != 0 {
                                playerKeys.menu_cancel_mut().fPressed = false;
                            }

                            iTeamMember += 1;
                        }

                        playerKeys = Ptr::from_mut(&mut game_values.playerInput.outputControls[game_values.teamids[ct][0] as usize]);

                        //If there are no AI moves to make
                        if iNextMove == -1 {
                            let mut fUsedItem = false;

                            //See if we are touching a door
                            let mut fDoor: [bool; 4] = [false, false, false, false];
                            g_worldmap.is_touching_door(iPlayerCurrentTile.x, iPlayerCurrentTile.y, &mut fDoor);

                            //See if we can use a key to keep going
                            let mut iPowerup: i16 = 0;
                            while iPowerup < game_values.worldpowerupcount[self.iControllingTeam as usize] {
                                let iType = game_values.worldpowerups[self.iControllingTeam as usize][iPowerup as usize] as i32;

                                if iType >= NUM_POWERUPS + 5 && iType <= NUM_POWERUPS + 8 {
                                    //Door Keys
                                    let iKeyType = iType - NUM_POWERUPS - 5;
                                    if fDoor[iKeyType as usize] {
                                        // iTeamMember is the member count left by the loop above, as in the C++.
                                        let iPlayer = teamids_flat(self.iControllingTeam, iTeamMember);
                                        fUsedItem = self.use_powerup(iPlayer, self.iControllingTeam, iPowerup, false);
                                    }
                                }
                                iPowerup += 1;
                            }

                            //if not, flag that there are no moves to make
                            self.fNoInterestingMoves = !fUsedItem;
                        }
                        if iNextMove == 0 {
                            playerKeys.menu_up_mut().fPressed = true;
                        } else if iNextMove == 1 {
                            playerKeys.menu_down_mut().fPressed = true;
                        } else if iNextMove == 2 {
                            playerKeys.menu_left_mut().fPressed = true;
                        } else if iNextMove == 3 {
                            playerKeys.menu_right_mut().fPressed = true;
                        } else if iNextMove == 4 {
                            self.pressSelectKeys = playerKeys;
                            self.iPressSelectTimer = 60;
                        }
                    } else {
                        self.iPressSelectTimer -= 1;
                        if self.iPressSelectTimer <= 0 {
                            self.iPressSelectTimer = 0;
                            let mut keys = self.pressSelectKeys;
                            keys.menu_select_mut().fPressed = true;
                        }
                    }
                } else {
                    self.iPressSelectTimer = 0;
                }
            }

            for iPlayer in 0..4i16 {
                let p = iPlayer as usize;
                let playerKeys: Ptr<COutputControl> = Ptr::from_mut(&mut game_values.playerInput.outputControls[p]);

                let iPlayerCurrentTile = g_worldmap.get_player_current_tile();

                let iTeamId = lookup_team_id(iPlayer);

                if self.iState == -1 && self.iNumPopups == 0 && !self.iPopupFlag[0] && !self.iPopupFlag[1] && !self.iPopupFlag[2] && !self.iPopupFlag[3] {
                    if self.iControllingTeam == lookup_team_id(iPlayer) && iPlayerState == 0 && game_values.playercontrol[p] > 0 {
                        //if this player is player or cpu
                        let tile: WorldMapTile = *g_worldmap.tiles.at(iPlayerCurrentTile.x as usize, iPlayerCurrentTile.y as usize);

                        let mut iTemp: i16 = 0; //Just a temp value so we can call the GetVehicleInPlayerTile method
                        let fVehicleInTile = g_worldmap.get_vehicle_in_player_tile(&mut iTemp) >= 0;
                        let (cx, cy) = (iPlayerCurrentTile.x, iPlayerCurrentTile.y);

                        if playerKeys.menu_up().fPressed || playerKeys.menu_up().fDown {
                            //Make sure there is a path connection and that there is no stage or vehicle blocking the way
                            if self.can_move(&tile, 0, 14, cx, cy - 1, 0, fVehicleInTile) {
                                if self.fUsingCloud && (tile.iCompleted == -2 || fVehicleInTile) && self.iReturnDirection != 0 {
                                    self.use_cloud(false);
                                }

                                g_worldmap.move_player(0);
                                iPlayerState = g_worldmap.get_player_state();

                                self.iReturnDirection = 1;

                                if_sound_on_play(&mut rm.sfx_worldmove);

                                //Start draw cycle over
                                self.restart_draw_cycle_if_needed();
                            } else if playerKeys.menu_up().fPressed {
                                if_sound_on_play(&mut rm.sfx_hit);
                            }
                        } else if playerKeys.menu_down().fPressed || playerKeys.menu_down().fDown {
                            if self.can_move(&tile, 1, 14, cx, cy + 1, 1, fVehicleInTile) {
                                if self.fUsingCloud && (tile.iCompleted == -2 || fVehicleInTile) && self.iReturnDirection != 1 {
                                    self.use_cloud(false);
                                }

                                g_worldmap.move_player(1);
                                iPlayerState = g_worldmap.get_player_state();

                                self.iReturnDirection = 0;

                                //Start draw cycle over
                                self.restart_draw_cycle_if_needed();

                                if_sound_on_play(&mut rm.sfx_worldmove);
                            } else if playerKeys.menu_down().fPressed {
                                if_sound_on_play(&mut rm.sfx_hit);
                            }
                        } else if playerKeys.menu_left().fPressed || playerKeys.menu_left().fDown {
                            if self.can_move(&tile, 2, 12, cx - 1, cy, 2, fVehicleInTile) {
                                if self.fUsingCloud && (tile.iCompleted == -2 || fVehicleInTile) && self.iReturnDirection != 2 {
                                    self.use_cloud(false);
                                }

                                g_worldmap.move_player(2);
                                iPlayerState = g_worldmap.get_player_state();

                                self.iReturnDirection = 3;

                                //Start draw cycle over
                                self.restart_draw_cycle_if_needed();

                                if_sound_on_play(&mut rm.sfx_worldmove);
                            } else {
                                g_worldmap.face_player(1);

                                if playerKeys.menu_left().fPressed {
                                    if_sound_on_play(&mut rm.sfx_hit);
                                }
                            }
                        } else if playerKeys.menu_right().fPressed || playerKeys.menu_right().fDown {
                            if self.can_move(&tile, 3, 12, cx + 1, cy, 3, fVehicleInTile) {
                                if self.fUsingCloud && (tile.iCompleted == -2 || fVehicleInTile) && self.iReturnDirection != 3 {
                                    self.use_cloud(false);
                                }

                                g_worldmap.move_player(3);
                                iPlayerState = g_worldmap.get_player_state();

                                self.iReturnDirection = 2;

                                //Start draw cycle over
                                self.restart_draw_cycle_if_needed();

                                if_sound_on_play(&mut rm.sfx_worldmove);
                            } else {
                                g_worldmap.face_player(0);

                                if playerKeys.menu_right().fPressed {
                                    if_sound_on_play(&mut rm.sfx_hit);
                                }
                            }
                        } else if playerInput.outputControls[p].menu_select().fPressed {
                            //Lookup current tile and see if it is a type of tile you can interact with
                            //If there is a vehicle on this tile, then load it's stage
                            let iStage = g_worldmap.get_vehicle_in_player_tile(&mut self.iVehicleId);
                            if iStage >= 0 && iStage < g_worldmap.iNumStages {
                                return self.init_game(iStage, iPlayer, fNeedAiControl);
                            }

                            //if it is a stage, then load the stage
                            let tile: WorldMapTile = *g_worldmap.tiles.at(iPlayerCurrentTile.x as usize, iPlayerCurrentTile.y as usize);

                            let iType = tile.iType - 6;
                            if iType >= 0 && iType < g_worldmap.iNumStages && tile.iCompleted == -2 {
                                return self.init_game(iType, iPlayer, fNeedAiControl);
                            }

                            if g_worldmap.get_warp_in_player_tile(&mut self.iWarpCol, &mut self.iWarpRow) {
                                self.iState = 4;
                                self.iScreenfade = 0;
                                self.iScreenfadeRate = 8;

                                if_sound_on_play(&mut rm.sfx_pipe);
                            }
                        }
                    }
                } else if self.state_transition(iTeamId) == 3 {
                    //not transitioning to or from the item popup menu
                    let t = iTeamId as usize;
                    if playerKeys.menu_up().fPressed {
                        if self.iItemPage[t] > 0 {
                            self.iItemPage[t] -= 1;
                            self.iItemCol[t] = 0;
                            if_sound_on_play(&mut rm.sfx_worldmove);
                        } else {
                            if_sound_on_play(&mut rm.sfx_hit);
                        }
                    } else if playerKeys.menu_down().fPressed {
                        if self.iItemPage[t] < 3 && (self.iItemPage[t] as i32 + 1) * 8 < game_values.worldpowerupcount[t] as i32 {
                            self.iItemPage[t] += 1;
                            self.iItemCol[t] = 0;
                            if_sound_on_play(&mut rm.sfx_worldmove);
                        } else {
                            if_sound_on_play(&mut rm.sfx_hit);
                        }
                    } else if playerKeys.menu_left().fPressed {
                        if self.iItemCol[t] > 0 {
                            self.iItemCol[t] -= 1;
                            if_sound_on_play(&mut rm.sfx_worldmove);
                        } else if self.iItemCol[t] == 0 && self.iItemPage[t] > 0 {
                            self.iItemCol[t] = 7;
                            self.iItemPage[t] -= 1;
                            if_sound_on_play(&mut rm.sfx_worldmove);
                        } else {
                            if_sound_on_play(&mut rm.sfx_hit);
                        }
                    } else if playerKeys.menu_right().fPressed {
                        if (self.iItemPage[t] as i32 * 8 + self.iItemCol[t] as i32 + 1) < game_values.worldpowerupcount[t] as i32 {
                            if self.iItemCol[t] < 7 {
                                self.iItemCol[t] += 1;
                                if_sound_on_play(&mut rm.sfx_worldmove);
                            } else if self.iItemCol[t] == 7 && self.iItemPage[t] < 3 {
                                self.iItemCol[t] = 0;
                                self.iItemPage[t] += 1;
                                if_sound_on_play(&mut rm.sfx_worldmove);
                            } else {
                                if_sound_on_play(&mut rm.sfx_hit);
                            }
                        }
                    } else if playerKeys.menu_select().fPressed {
                        if game_values.worldpowerupcount[t] > 0 {
                            let iIndex = self.iItemPage[t] * 8 + self.iItemCol[t];
                            self.use_powerup(iPlayer, iTeamId, iIndex, true);
                        }
                    }
                }

                if self.iState == -1 && (self.state_transition(iTeamId) == 0 || self.state_transition(iTeamId) == 3) {
                    let t = iTeamId as usize;
                    if playerKeys.menu_cancel().fPressed {
                        if DEVICE_KEYBOARD != playerInput.inputControls[p].iDevice || iPlayer == 0 {
                            self.fModifying = false;
                            return MENU_CODE_BACK_TEAM_SELECT_MENU;
                        }
                    }

                    if (game_values.playercontrol[p] == 1 || self.fNoInterestingMoves)
                        && (self.iPopupFlag[t]
                            || playerKeys.menu_random().fPressed
                            || (iPlayer != 0 && playerInput.inputControls[p].iDevice == DEVICE_KEYBOARD && playerKeys.menu_cancel().fPressed))
                    {
                        self.iPopupFlag[t] = true;

                        if iPlayerState == 0 && self.iStateTransition[t] == 0 {
                            self.iPopupFlag[t] = false;

                            self.iPopupOrder[self.iNumPopups as usize] = iTeamId;
                            self.iPopupOffsets[t] = self.iNumPopups << 6;
                            self.iNumPopups += 1;
                            self.iPopupOffsetsCurrent[t] = self.iPopupOffsets[t];

                            self.iStateTransition[t] = 1;

                            self.iItemPage[t] = 0;
                            self.iItemCol[t] = 0;

                            if_sound_on_play(&mut rm.sfx_inventory);
                        } else if self.iStateTransition[t] == 3 {
                            self.iPopupFlag[t] = false;

                            self.iStateTransition[t] = 2;
                            if_sound_on_play(&mut rm.sfx_inventory);
                        }
                    }
                }
            }

            MENU_CODE_NONE
        }
    }

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }
}
