//! Port of src/smw/ui/MI_TeamSelect.cpp

use crate::common::global_constants::ticks;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::game::App;
use crate::common::gfx::gfx_setjoystickteamcolor;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::input::{CPlayerInput, COutputControl, DEVICE_KEYBOARD};
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_image::MI_Image;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;

pub struct MI_TeamSelect {
    pub ui_control: UI_Control,

    pub miImage: Box<MI_Image>,
    pub spr: SpriteId,

    pub iTeamIDs: [[i16; 3]; 4],
    pub iTeamCounts: [i16; 4],
    pub iNumTeams: i16,

    pub iAnimationTimer: i16,
    pub iAnimationFrame: i16,
    pub iRandomAnimationFrame: i16,

    pub fReady: [bool; 4],
    pub fAllReady: bool,

    pub iFastScroll: [i16; 4],
    pub iFastScrollTimer: [i16; 4],
}
crate::impl_base!(MI_TeamSelect => ui_control: UI_Control);

impl MI_TeamSelect {
    pub fn new(g: &mut Game, spr_background_ref: SpriteId, x: i16, y: i16) -> Self {
        let ui_control = UI_Control::new(x, y);
        let spr = spr_background_ref;
        let miImage = Box::new(MI_Image::new(spr, ui_control.m_pos.x, ui_control.m_pos.y, 0, 0, 416, 256, 1, 1, 0));

        let mut this = MI_TeamSelect {
            ui_control,
            miImage,
            spr,
            iTeamIDs: [[0; 3]; 4],
            iTeamCounts: [0; 4],
            iNumTeams: 0,
            iAnimationTimer: 0,
            iAnimationFrame: 0,
            iRandomAnimationFrame: 0,
            fReady: [false; 4],
            fAllReady: false,
            iFastScroll: [0; 4],
            iFastScrollTimer: [0; 4],
        };

        for iTeam in 0..4usize {
            this.iTeamCounts[iTeam] = g.game_values.teamcounts[iTeam];

            for iSlot in 0..3usize {
                this.iTeamIDs[iTeam][iSlot] = g.game_values.teamids[iTeam][iSlot];
            }

            this.fReady[iTeam] = false;
        }

        this.iAnimationTimer = 0;
        this.iAnimationFrame = 0;
        this.iRandomAnimationFrame = 0;
        this
    }

    fn find_new_team(&mut self, g: &mut Game, iPlayerID: i16, iDirection: i16) {
        let mut gp = g.ptr();
        for iTeam in 0..4i16 {
            let mut iTeamItem: i16 = 0;
            while iTeamItem < self.iTeamCounts[iTeam as usize] {
                if self.iTeamIDs[iTeam as usize][iTeamItem as usize] == iPlayerID {
                    self.iTeamCounts[iTeam as usize] -= 1;

                    let mut iMovePlayer = iTeamItem as i32;
                    while iMovePlayer < self.iTeamCounts[iTeam as usize] as i32 {
                        self.iTeamIDs[iTeam as usize][iMovePlayer as usize] = self.iTeamIDs[iTeam as usize][(iMovePlayer + 1) as usize];
                        iMovePlayer += 1;
                    }

                    let mut iNewTeam: i16 = iTeam;
                    let mut fOnlyTeam: bool;

                    loop {
                        iNewTeam += iDirection;

                        if iNewTeam < 0 {
                            iNewTeam = 3;
                        } else if iNewTeam > 3 {
                            iNewTeam = 0;
                        }

                        fOnlyTeam = true;
                        for iMovePlayer in 0..4i32 {
                            if iMovePlayer == iNewTeam as i32 {
                                continue;
                            }

                            if self.iTeamCounts[iMovePlayer as usize] > 0 {
                                fOnlyTeam = false;
                                break;
                            }
                        }
                        if !fOnlyTeam {
                            break;
                        }
                    }

                    self.iTeamIDs[iNewTeam as usize][self.iTeamCounts[iNewTeam as usize] as usize] = iPlayerID;
                    self.iTeamCounts[iNewTeam as usize] += 1;

                    if g.game_values.teamcolors {
                        g.game_values.colorids[iPlayerID as usize] = iNewTeam;

                        //Skip skins that are invalid
                        while !gp.get().rm.load_menu_skin(g, iPlayerID, gp.get().game_values.skinids[iPlayerID as usize], iNewTeam, false) {
                            g.game_values.skinids[iPlayerID as usize] += 1;
                            if g.game_values.skinids[iPlayerID as usize] as usize >= g.skinlist.count() {
                                g.game_values.skinids[iPlayerID as usize] = 0;
                            }
                        }
                    }

                    return;
                }
                iTeamItem += 1;
            }
        }
    }

    pub fn reset(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        for iPlayer in 0..4i16 {
            let mut iTeamID: i16 = 0;
            let mut iSlotID: i16 = 0;
            let mut fFound = false;
            while iTeamID < 4 {
                iSlotID = 0;
                while iSlotID < self.iTeamCounts[iTeamID as usize] {
                    if self.iTeamIDs[iTeamID as usize][iSlotID as usize] == iPlayer {
                        fFound = true;
                        break;
                    }
                    iSlotID += 1;
                }

                if fFound {
                    break;
                }
                iTeamID += 1;
            }

            if fFound {
                //Need to remove the player
                if g.game_values.playercontrol[iPlayer as usize] == 0 {
                    self.iTeamCounts[iTeamID as usize] -= 1;

                    if self.iTeamCounts[iTeamID as usize] > iSlotID {
                        let mut iSlot = iSlotID;
                        while iSlot < self.iTeamCounts[iTeamID as usize] {
                            self.iTeamIDs[iTeamID as usize][iSlot as usize] = self.iTeamIDs[iTeamID as usize][(iSlot + 1) as usize];
                            iSlot += 1;
                        }
                    }
                }
            } else {
                //A new player was added so find a spot for him
                if g.game_values.playercontrol[iPlayer as usize] > 0 {
                    let mut iLookForNewTeam = iPlayer;

                    while self.iTeamCounts[iLookForNewTeam as usize] >= 3 {
                        iLookForNewTeam += 1;
                        if iLookForNewTeam >= 4 {
                            iLookForNewTeam = 0;
                        }
                    }

                    self.iTeamIDs[iLookForNewTeam as usize][self.iTeamCounts[iLookForNewTeam as usize] as usize] = iPlayer;
                    self.iTeamCounts[iLookForNewTeam as usize] += 1;

                    if g.game_values.teamcolors {
                        g.game_values.colorids[iPlayer as usize] = iLookForNewTeam;
                    }
                }
            }
        }

        //Check to see if there is only one team and if so, split them up

        let mut iCountTeams: i16 = 0;
        let mut iLastTeam: i16 = 0;
        for iTeamID in 0..4i16 {
            if self.iTeamCounts[iTeamID as usize] > 0 {
                iCountTeams += 1;
                iLastTeam = iTeamID;
            }
        }

        if iCountTeams == 1 {
            let mut iLookForNewTeam = iLastTeam;
            iLookForNewTeam += 1;
            if iLookForNewTeam >= 4 {
                iLookForNewTeam = 0;
            }

            self.iTeamCounts[iLastTeam as usize] -= 1;
            let iPlayer = self.iTeamIDs[iLastTeam as usize][self.iTeamCounts[iLastTeam as usize] as usize];
            self.iTeamIDs[iLookForNewTeam as usize][self.iTeamCounts[iLookForNewTeam as usize] as usize] = iPlayer;
            self.iTeamCounts[iLookForNewTeam as usize] += 1;

            if g.game_values.teamcolors {
                g.game_values.colorids[iPlayer as usize] = iLookForNewTeam;
            }
        }

        self.iAnimationTimer = 0;
        self.iAnimationFrame = 0;

        self.fAllReady = true;

        for iPlayer in 0..4i16 {
            let p = iPlayer as usize;
            self.iFastScroll[p] = 0;
            self.iFastScrollTimer[p] = 0;

            if g.game_values.playercontrol[p] == 1 {
                self.fReady[p] = false;
                self.fAllReady = false;
            } else {
                self.fReady[p] = true;
            }

            if g.game_values.playercontrol[p] == 0 {
                continue;
            }

            if g.game_values.teamcolors {
                g.game_values.colorids[p] = self.get_team(iPlayer);
            } else {
                g.game_values.colorids[p] = iPlayer;
            }

            //Skip skins that are invalid
            while !gp.get().rm.load_menu_skin(g, iPlayer, gp.get().game_values.skinids[p], gp.get().game_values.colorids[p], false) {
                g.game_values.skinids[p] += 1;
                if g.game_values.skinids[p] as usize >= g.skinlist.count() {
                    g.game_values.skinids[p] = 0;
                }
            }
        }
    }

    pub fn organize_teams(&mut self, g: &mut Game) -> i16 {
        self.iNumTeams = 0;
        for iTeam in 0..4usize {
            g.game_values.teamcounts[iTeam] = 0;

            if self.iTeamCounts[iTeam] > 0 {
                for iTeamSpot in 0..3usize {
                    g.game_values.teamids[self.iNumTeams as usize][iTeamSpot] = self.iTeamIDs[iTeam][iTeamSpot];
                }

                g.game_values.teamcounts[self.iNumTeams as usize] = self.iTeamCounts[iTeam];
                self.iNumTeams += 1;
            }
        }

        self.iNumTeams
    }

    pub fn get_team(&self, iPlayerID: i16) -> i16 {
        for iTeam in 0..4i16 {
            for iSlot in 0..self.iTeamCounts[iTeam as usize] {
                if self.iTeamIDs[iTeam as usize][iSlot as usize] == iPlayerID {
                    return iTeam;
                }
            }
        }

        -1
    }
}

impl UI_ControlTrait for MI_TeamSelect {
    crate::impl_ctl!();

    fn update(&mut self, g: &mut Game) {
        self.iAnimationTimer += 1;
        if self.iAnimationTimer > 7 {
            self.iAnimationTimer = 0;

            self.iAnimationFrame += 2;
            if self.iAnimationFrame > 2 {
                self.iAnimationFrame = 0;
            }

            self.iRandomAnimationFrame += 32;
            if self.iRandomAnimationFrame >= 128 {
                self.iRandomAnimationFrame = 0;
            }
        }
    }

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if !self.m_visible {
            return;
        }

        self.miImage.draw(g);

        let mut iPlayerCount: i16 = 0;

        for iPlayer in 0..4usize {
            if g.game_values.playercontrol[iPlayer] > 0 {
                iPlayerCount += 1;
            }
        }

        let px = self.m_pos.x as i32;
        let py = self.m_pos.y as i32;

        for iTeam in 0..4i32 {
            for iTeamItem in 0..self.iTeamCounts[iTeam as usize] as i32 {
                let iPlayerID = self.iTeamIDs[iTeam as usize][iTeamItem as usize];
                let pid = iPlayerID as usize;

                if g.game_values.randomskin[pid] {
                    g.rm.sprite(self.spr).draw_src(
                        g,
                        iTeam * 96 + 43 + px,
                        iTeamItem * 36 + 52 + py,
                        &Rect { x: 416, y: if self.fReady[pid] { 0 } else { self.iRandomAnimationFrame as i32 }, w: 42, h: 32 },
                    );
                } else {
                    let frame = if self.fReady[pid] { 0 } else { self.iAnimationFrame as usize };
                    gp.rm.sprite(gp.rm.spr_player[pid][frame]).draw_src(g, iTeam * 96 + 48 + px, iTeamItem * 36 + 52 + py, &Rect { x: 0, y: 0, w: 32, h: 32 });
                }

                gp.rm.sprite(gp.rm.spr_menu_boxed_numbers).draw_src(
                    g,
                    iTeam * 96 + 44 + px,
                    iTeamItem * 36 + 72 + py,
                    &Rect { x: iPlayerID as i32 * 16, y: gp.game_values.colorids[pid] as i32 * 16, w: 16, h: 16 },
                );
            }

            let t = iTeam as usize;
            if g.game_values.playercontrol[t] > 0 {
                gp.rm.sprite(gp.rm.spr_player_select_ready).draw_src(g, iTeam * 160 + 16, 368, &Rect { x: 0, y: 0, w: 128, h: 96 });

                gp.rm.sprite(gp.rm.spr_menu_boxed_numbers).draw_src(g, iTeam * 160 + 32, 388, &Rect { x: iTeam * 16, y: gp.game_values.colorids[t] as i32 * 16, w: 16, h: 16 });
                let skinlist = g.skinlist;
                let name: &str = if g.game_values.randomskin[t] { "Random" } else { &skinlist.at(g.game_values.skinids[t] as usize).name };
                gp.rm.menu_font_small.draw_chop_right(g, iTeam * 160 + 52, 404 - gp.rm.menu_font_small.get_height(), 80, name);

                let srcy = if !self.fReady[t] {
                    0
                } else if g.game_values.playercontrol[t] == 1 {
                    32
                } else {
                    64
                };
                gp.rm.sprite(gp.rm.spr_player_select_ready).draw_src(g, iTeam * 160 + 64, 408, &Rect { x: 128, y: srcy, w: 34, h: 32 });
            }
        }
        let _ = iPlayerCount;

        if self.fAllReady {
            let fy = (py as f32 + App::screenHeight as f32 * 0.47f32) as i32;
            gp.rm.sprite(gp.rm.menu_plain_field).draw_src(g, px + 108, fy, &Rect { x: 0, y: 160, w: 100, h: 32 });
            gp.rm.sprite(gp.rm.menu_plain_field).draw_src(g, px + 208, fy, &Rect { x: 412, y: 160, w: 100, h: 32 });
            gp.rm.menu_font_large.draw_centered(g, App::screenWidth / 2, (py as f32 + App::screenHeight as f32 * 0.48f32) as i32, "Continue");
        }
    }

    fn send_input(&mut self, g: &mut Game, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        let mut gp = g.ptr();
        {
            for iPlayer in 0..4i16 {
                let p = iPlayer as usize;
                let playerKeys: Ptr<COutputControl> = Ptr::from_mut(&mut g.game_values.playerInput.outputControls[p]);

                if g.game_values.playercontrol[p] > 0 && !self.fReady[p] {
                    //if this player is player or cpu
                    if playerKeys.menu_left().fPressed {
                        if playerKeys.menu_right().fDown {
                            g.game_values.randomskin[p] = !g.game_values.randomskin[p];
                        } else {
                            self.find_new_team(g, iPlayer, -1);
                        }
                    }

                    if playerKeys.menu_right().fPressed {
                        if playerKeys.menu_left().fDown {
                            g.game_values.randomskin[p] = !g.game_values.randomskin[p];
                        } else {
                            self.find_new_team(g, iPlayer, 1);
                        }
                    }

                    //Scroll up/down through player skins
                    if !g.game_values.randomskin[p] {
                        if playerKeys.menu_up().fPressed {
                            loop {
                                if playerKeys.menu_down().fDown {
                                    g.game_values.skinids[p] = RANDOM_INT(gp.get(), g.skinlist.count() as i32) as i16;
                                } else {
                                    g.game_values.skinids[p] -= 1;
                                    if g.game_values.skinids[p] < 0 {
                                        g.game_values.skinids[p] = g.skinlist.count() as i16 - 1;
                                    }
                                }
                                if gp.get().rm.load_menu_skin(g, iPlayer, gp.get().game_values.skinids[p], gp.get().game_values.colorids[p], false) {
                                    break;
                                }
                            }
                        } else if playerKeys.menu_up().fDown {
                            if self.iFastScroll[p] == 0 {
                                self.iFastScrollTimer[p] += 1;
                                if self.iFastScrollTimer[p] > ticks(40) as i16 {
                                    self.iFastScroll[p] = 1;
                                }
                            } else {
                                self.iFastScrollTimer[p] += 1;
                                if self.iFastScrollTimer[p] > 5 {
                                    loop {
                                        g.game_values.skinids[p] -= 1;
                                        if g.game_values.skinids[p] < 0 {
                                            g.game_values.skinids[p] = g.skinlist.count() as i16 - 1;
                                        }
                                        if gp.get().rm.load_menu_skin(g, iPlayer, gp.get().game_values.skinids[p], gp.get().game_values.colorids[p], false) {
                                            break;
                                        }
                                    }

                                    self.iFastScrollTimer[p] = 0;
                                }
                            }
                        }

                        if playerKeys.menu_down().fPressed {
                            loop {
                                if playerKeys.menu_up().fDown {
                                    g.game_values.skinids[p] = RANDOM_INT(gp.get(), g.skinlist.count() as i32) as i16;
                                } else {
                                    g.game_values.skinids[p] += 1;
                                    if g.game_values.skinids[p] as usize >= g.skinlist.count() {
                                        g.game_values.skinids[p] = 0;
                                    }
                                }
                                if gp.get().rm.load_menu_skin(g, iPlayer, gp.get().game_values.skinids[p], gp.get().game_values.colorids[p], false) {
                                    break;
                                }
                            }
                        } else if playerKeys.menu_down().fDown {
                            if self.iFastScroll[p] == 0 {
                                self.iFastScrollTimer[p] += 1;
                                if self.iFastScrollTimer[p] > ticks(40) as i16 {
                                    self.iFastScroll[p] = 1;
                                }
                            } else {
                                self.iFastScrollTimer[p] += 1;
                                if self.iFastScrollTimer[p] > 5 {
                                    loop {
                                        g.game_values.skinids[p] += 1;
                                        if g.game_values.skinids[p] as usize >= g.skinlist.count() {
                                            g.game_values.skinids[p] = 0;
                                        }
                                        if gp.get().rm.load_menu_skin(g, iPlayer, gp.get().game_values.skinids[p], gp.get().game_values.colorids[p], false) {
                                            break;
                                        }
                                    }

                                    self.iFastScrollTimer[p] = 0;
                                }
                            }
                        }

                        if (!playerKeys.menu_up().fDown && !playerKeys.menu_down().fDown) || (playerKeys.menu_up().fDown && playerKeys.menu_down().fDown) {
                            self.iFastScroll[p] = 0;
                            self.iFastScrollTimer[p] = 0;
                        }
                    } else {
                        self.iFastScroll[p] = 0;
                        self.iFastScrollTimer[p] = 0;
                    }

                    if playerKeys.menu_random().fPressed {
                        if playerKeys.menu_scrollfast().fDown {
                            g.game_values.randomskin[p] = !g.game_values.randomskin[p];
                        } else if !g.game_values.randomskin[p] {
                            loop {
                                g.game_values.skinids[p] = RANDOM_INT(gp.get(), g.skinlist.count() as i32) as i16;
                                if gp.get().rm.load_menu_skin(g, iPlayer, gp.get().game_values.skinids[p], gp.get().game_values.colorids[p], false) {
                                    break;
                                }
                            }
                        }
                    }
                }

                if playerInput.outputControls[p].menu_select().fPressed {
                    self.fReady[p] = true;

                    if self.fAllReady && (DEVICE_KEYBOARD != g.input_control(playerInput.inputControls[p]).iDevice || iPlayer == 0) {
                        self.fModifying = false;
                        println!("MI_TeamSelect::SendInput MENU_CODE_TO_GAME_SETUP_MENU");
                        return MENU_CODE_TO_GAME_SETUP_MENU;
                    }

                    self.fAllReady = true;
                    for i in 0..4usize {
                        if !self.fReady[i] {
                            self.fAllReady = false;
                            break;
                        }
                    }
                }

                if playerInput.outputControls[p].menu_cancel().fPressed {
                    if g.game_values.playercontrol[p] > 0 && self.fReady[p] {
                        self.fReady[p] = false;
                        self.fAllReady = false;
                    } else if DEVICE_KEYBOARD != g.input_control(playerInput.inputControls[p]).iDevice || iPlayer == 0 {
                        self.fModifying = false;
                        return MENU_CODE_BACK_TO_MATCH_SELECTION_MENU;
                    }
                }

                if DEVICE_KEYBOARD != g.input_control(playerInput.inputControls[p]).iDevice {
                    let team = self.get_team(iPlayer);
                    gfx_setjoystickteamcolor(
                        g,
                        g.input_control(playerInput.inputControls[p]).iDevice as i32,
                        team,
                        if self.fReady[p] { 1.0f32 } else { 0.5f32 },
                    );
                }
            }
        }

        MENU_CODE_NONE
    }

    fn modify(&mut self, g: &mut Game, modify: bool) -> MenuCodeEnum {
        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }

    /// Not in upstream: a tap on a player's box readies that player, and a tap on Continue starts setup.
    /// Every tap counts as on this control, which fills the screen and only takes input while modifying.
    fn mouse_click(&mut self, g: &mut Game, iMouseX: i16, iMouseY: i16) -> MenuCodeEnum {
        if !self.m_visible {
            return MENU_CODE_NONE;
        }

        let (mx, my) = (iMouseX as i32, iMouseY as i32);
        let (px, py) = (self.m_pos.x as i32, self.m_pos.y as i32);

        if self.fAllReady {
            let fy = (py as f32 + App::screenHeight as f32 * 0.47f32) as i32;
            if mx >= px + 108 && mx < px + 308 && my >= fy && my < fy + 32 {
                self.fModifying = false;
                return MENU_CODE_TO_GAME_SETUP_MENU;
            }
        }

        for p in 0..4usize {
            let x = p as i32 * 160 + 16;
            if g.game_values.playercontrol[p] > 0 && mx >= x && mx < x + 128 && my >= 368 && my < 464 {
                self.fReady[p] = !self.fReady[p];
                self.fAllReady = self.fReady.iter().all(|&r| r);
            }
        }

        MENU_CODE_CLICKED
    }
}
