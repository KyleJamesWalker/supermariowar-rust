//! The bonus wheel: powerups appear around a ring, a selector spins, and the winners get where it stops.

use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::global_constants::{acc, ticks, vel, NUM_POWERUPS, PGFX_JUMPING_R, TICKS_PER_SECOND, TWO_PI};
use crate::common::math::trig::{cosf, sinf};
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::uicontrol::{TextAlign, UI_Control, UI_ControlTrait};
use crate::globals::*;

/// Powerup slots around the wheel.
pub const NUMBONUSITEMSONWHEEL: i32 = 10;
const NB: usize = NUMBONUSITEMSONWHEEL as usize;

/// Ten powerups around a ring with a spinning selector; Continue appears once it stops.
pub struct MI_BonusWheel {
    pub ui_control: UI_Control,

    /// 0 while the powerups appear one by one, 1 once the wheel spins.
    iState: i16,
    /// Slots revealed so far.
    iDisplayPowerupIndex: i16,
    iDisplayPowerupTimer: i16,

    miBonusImages: [Box<MI_Image>; NB],
    miPlayerImages: Vec<Box<MI_Image>>,

    miContinueButton: Box<MI_Button>,

    /// Powerup in each slot.
    iChosenPowerups: [i16; NB],

    /// Frames spun without a press; the wheel stops itself after 10 seconds.
    iPressSelectTimer: i16,
    /// The stop was pressed and the selector is winding down.
    fPressedSelect: bool,
    /// The wheel stopped and the winners got the powerup.
    fPowerupSelectionDone: bool,

    iSelectorAnimation: i16,
    iSelectorAnimationCounter: i16,

    /// Selector speed in radians per frame.
    dSelectionSpeed: f32,
    /// Selector angle in radians, 0 to `TWO_PI`.
    dSelectionAngle: f32,
    /// Powerup the selector will stop on, set when the stop is pressed.
    iSelectedPowerup: i16,
    /// Index into `dSelectionSector` of the next boundary that plays a tick.
    iNextSelectionSoundIndex: i16,
    /// Start angle of each slot in radians, then `TWO_PI` closing the last.
    dSelectionSector: [f32; NB + 1],

    /// Per-frame slowdown that lands the selector on `iSelectedPowerup`.
    dSelectionWinddownSpeed: f32,

    /// Speed the free spin drifts toward; rerolled when `iSelectionSpeedTimer` runs out.
    dSelectionSpeedGoal: f32,
    iSelectionSpeedTimer: i16,

    iNumPlayers: i16,
    /// Team that spins and receives the powerup.
    iWinningTeam: i16,

    /// The winning team has no humans, so the wheel stops at once.
    fCpuControlled: bool,
}
crate::impl_base!(MI_BonusWheel => ui_control: UI_Control);

impl MI_BonusWheel {
    pub fn new(g: &mut Game, x: i16, y: i16) -> Self {
        let mut gp = g.ptr();
        let ui_control = UI_Control::new(x, y);
        let mut dSelectionSector = [0.0f32; NB + 1];

        let miBonusImages: [Box<MI_Image>; NB] = std::array::from_fn(|iImage| {
            dSelectionSector[iImage] = iImage as f32 * TWO_PI / NUMBONUSITEMSONWHEEL as f32;

            let iPowerupX = (x as i32 + 160 + (110.0f32 * cosf(dSelectionSector[iImage])) as i16 as i32) as i16;
            let iPowerupY = (y as i32 + 208 + (110.0f32 * sinf(dSelectionSector[iImage])) as i16 as i32) as i16;

            Box::new(MI_Image::new(g.rm.spr_storedpoweruplarge, iPowerupX, iPowerupY, 0, 0, 32, 32, 1, 1, 0))
        });

        //Fix the last sector to allow correct detection of sector for tick sound
        dSelectionSector[NB] = TWO_PI;

        let mut miContinueButton = Box::new(MI_Button::new(
            g,
            gp.rm.menu_plain_field,
            ui_control.m_pos.x + 76,
            ui_control.m_pos.y + 390,
            "Continue",
            200,
            TextAlign::CENTER,
        ));
        miContinueButton.set_visible(false);
        miContinueButton.set_code(MENU_CODE_BONUS_DONE);

        MI_BonusWheel {
            ui_control,
            iState: 0,
            iDisplayPowerupIndex: 0,
            iDisplayPowerupTimer: 0,
            miBonusImages,
            miPlayerImages: Vec::new(),
            miContinueButton,
            iChosenPowerups: [0; NB],
            iPressSelectTimer: 0,
            fPressedSelect: false,
            fPowerupSelectionDone: false,
            iSelectorAnimation: 0,
            iSelectorAnimationCounter: 0,
            dSelectionSpeed: 0.0,
            dSelectionAngle: 0.0,
            iSelectedPowerup: 0,
            iNextSelectionSoundIndex: 0,
            dSelectionSector,
            dSelectionWinddownSpeed: 0.0,
            dSelectionSpeedGoal: 0.0,
            iSelectionSpeedTimer: 0,
            iNumPlayers: 0,
            iWinningTeam: 0,
            fCpuControlled: false,
        }
    }

    /// True once the wheel stopped and the winners got the powerup.
    pub fn get_powerup_selection_done(&self) -> bool {
        self.fPowerupSelectionDone
    }

    /// Fills the wheel from the item weights, one slot a poison mushroom, for the match winner or with `fTournament` the tournament winner.
    pub fn reset(&mut self, g: &mut Game, fTournament: bool) {
        let mut gp = g.ptr();
        //Setup the state so that we make powerups appear one by one before the wheel starts spinning
        self.iState = 0;
        self.iDisplayPowerupIndex = 0;
        self.iDisplayPowerupTimer = 0;

        if fTournament {
            self.iWinningTeam = g.game_values.tournamentwinner;
        } else {
            self.iWinningTeam = g.mode().gm().winningteam;
        }

        //Randomly display the powerups around the ring
        let mut iCountWeight: i16 = 0;
        for iPowerup in 0..NUM_POWERUPS as usize {
            iCountWeight += g.game_values.powerupweights[iPowerup];
        }

        //Always have at least 1 poison mushroom to try to avoid
        let iPoisonMushroom = RANDOM_INT(gp.get(), NUMBONUSITEMSONWHEEL) as i16;

        for iPowerup in 0..NUMBONUSITEMSONWHEEL as i16 {
            let mut iChoosePowerup: i32 = 0;

            if iCountWeight > 0 && iPoisonMushroom != iPowerup {
                let iRandPowerup = RANDOM_INT(gp.get(), iCountWeight as i32 + 1);
                let mut iPowerupWeightCount: i32 = g.game_values.powerupweights[iChoosePowerup as usize] as i32;

                while iPowerupWeightCount < iRandPowerup {
                    iChoosePowerup += 1;
                    iPowerupWeightCount += g.game_values.powerupweights[iChoosePowerup as usize] as i32;
                }
            }

            self.miBonusImages[iPowerup as usize].set_image((iChoosePowerup << 5) as i16, 0, 32, 32);
            self.iChosenPowerups[iPowerup as usize] = iChoosePowerup as i16;
        }

        //Setup player images on wheel
        self.miPlayerImages.clear();

        self.iNumPlayers = g.game_values.teamcounts[self.iWinningTeam as usize];

        self.miPlayerImages = Vec::with_capacity(self.iNumPlayers.max(0) as usize);

        let mut iPlayerX = (self.m_pos.x as i32 + 160 - ((self.iNumPlayers as i32 - 1) * 17)) as i16;
        for iPlayer in 0..self.iNumPlayers as usize {
            let id = g.game_values.teamids[self.iWinningTeam as usize][iPlayer] as usize;
            self.miPlayerImages.push(Box::new(MI_Image::new(
                g.rm.spr_player[id][PGFX_JUMPING_R as usize],
                iPlayerX,
                self.m_pos.y + 210,
                0,
                0,
                32,
                32,
                1,
                1,
                0,
            )));
            iPlayerX += 34;
        }

        //Indicate that the player hasn't choosen a powerup yet
        self.iPressSelectTimer = 0;
        self.fPressedSelect = false;
        self.fPowerupSelectionDone = false;
        self.miContinueButton.set_visible(false);

        //Counters to animate the selector's wings
        self.iSelectorAnimation = 0;
        self.iSelectorAnimationCounter = 0;

        //Figure out the initial position and speed of the selector
        self.dSelectionSpeed = vel((RANDOM_INT(gp.get(), 100) + 200) as f32 * 0.0005f32);
        self.dSelectionAngle = RANDOM_INT(gp.get(), NUMBONUSITEMSONWHEEL) as f32 * TWO_PI / NUMBONUSITEMSONWHEEL as f32;
        self.dSelectionSpeedGoal = vel((RANDOM_INT(gp.get(), 100) + 200) as f32 * 0.0005f32);
        self.iSelectionSpeedTimer = 0;

        for iSector in 0..NB {
            if self.dSelectionAngle > self.dSelectionSector[iSector] {
                self.iNextSelectionSoundIndex = iSector as i16 + 1;
            } else {
                break;
            }
        }

        //Figure out if only cpus are on the winning team, if so, the wheel will be stopped early
        self.fCpuControlled = true;
        for iPlayer in 0..g.game_values.teamcounts[self.iWinningTeam as usize] as usize {
            if g.game_values.playercontrol[g.game_values.teamids[self.iWinningTeam as usize][iPlayer] as usize] == 1 {
                self.fCpuControlled = false;
            }
        }
    }
}

impl UI_ControlTrait for MI_BonusWheel {
    crate::impl_ctl!();

    fn modify(&mut self, g: &mut Game, fModify: bool) -> MenuCodeEnum {
        if fModify && !self.fPressedSelect {
            self.fPressedSelect = true;

            let dNumWinddownSteps: f32 = self.dSelectionSpeed / acc(0.0005) - 1.0;
            let dWinddownAngle: f32 = self.dSelectionSpeed / 2.0f32 * dNumWinddownSteps;
            let mut dFinalAngle: f32 = self.dSelectionAngle + dWinddownAngle;

            //Bring the radians back down to between 0 and TWO_PI to do comparisons to the powerups on the wheel
            while dFinalAngle > TWO_PI {
                dFinalAngle -= TWO_PI;
            }

            let dSectorSize: f32 = TWO_PI / NUMBONUSITEMSONWHEEL as f32;
            for iSector in 0..NUMBONUSITEMSONWHEEL {
                if dFinalAngle >= iSector as f32 * dSectorSize && dFinalAngle < (iSector + 1) as f32 * dSectorSize {
                    self.iSelectedPowerup = self.iChosenPowerups[if iSector + 1 >= NUMBONUSITEMSONWHEEL { 0 } else { (iSector + 1) as usize }];
                    let dNewWinddownAngle: f32 = dWinddownAngle + (iSector + 1) as f32 * dSectorSize - dFinalAngle;

                    //Determine the speed we need to exactly hit the selected powerup when the selector winds down
                    self.dSelectionWinddownSpeed = self.dSelectionSpeed / (dNewWinddownAngle * 2.0f32 / self.dSelectionSpeed + 1.0f32);
                    break;
                }
            }
        }

        if self.fPowerupSelectionDone {
            return self.miContinueButton.modify(g, fModify);
        }

        MENU_CODE_NONE
    }

    fn update(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if self.iState == 0 {
            self.iDisplayPowerupTimer -= 1;
            if self.iDisplayPowerupTimer <= 0 {
                self.iDisplayPowerupTimer = ticks(20) as i16;

                let sector = self.dSelectionSector[self.iDisplayPowerupIndex as usize];
                let iPoofX = (self.m_pos.x as i32 + 152 + (110.0f32 * cosf(sector)) as i16 as i32) as i16;
                let iPoofY = (self.m_pos.y as i32 + 200 + (110.0f32 * sinf(sector)) as i16 as i32) as i16;

                let _ = (iPoofX, iPoofY);
                // TODO(eyecandy): draw a poof at iPoofX, iPoofY as each powerup appears.

                if_sound_on_play(g, &mut gp.rm.sfx_cannon);

                self.iDisplayPowerupIndex += 1;
                if self.iDisplayPowerupIndex as i32 >= NUMBONUSITEMSONWHEEL {
                    self.iState = 1;
                }
            }
        } else {
            if !self.fPressedSelect && (self.fCpuControlled || {
                self.iPressSelectTimer += 1;
                self.iPressSelectTimer > 10 * TICKS_PER_SECOND as i16
            }) {
                self.modify(g, true);
            }

            for iImage in 0..NB {
                self.miBonusImages[iImage].update(g);
            }

            for iPlayer in 0..self.iNumPlayers as usize {
                self.miPlayerImages[iPlayer].update(g);
            }

            self.miContinueButton.update(g);

            self.iSelectorAnimationCounter += 1;
            if self.iSelectorAnimationCounter > 8 {
                self.iSelectorAnimationCounter = 0;

                self.iSelectorAnimation += 1;
                if self.iSelectorAnimation > 1 {
                    self.iSelectorAnimation = 0;
                }
            }

            if self.iSelectionSpeedTimer > 0 {
                self.iSelectionSpeedTimer -= 1;
                if self.iSelectionSpeedTimer <= 0 {
                    self.dSelectionSpeedGoal = vel((RANDOM_INT(gp.get(), 100) + 200) as f32 * 0.0005f32);
                    self.iSelectionSpeedTimer = 0;
                }
            }

            if self.fPressedSelect {
                self.dSelectionSpeed -= self.dSelectionWinddownSpeed;
            } else if self.dSelectionSpeed < self.dSelectionSpeedGoal {
                self.dSelectionSpeed += acc(0.0005);

                if self.dSelectionSpeed >= self.dSelectionSpeedGoal {
                    self.dSelectionSpeed = self.dSelectionSpeedGoal;
                    self.iSelectionSpeedTimer = ticks(RANDOM_INT(gp.get(), 60) + 30) as i16;
                }
            } else if self.dSelectionSpeed > self.dSelectionSpeedGoal {
                self.dSelectionSpeed -= acc(0.0005);

                if self.dSelectionSpeed <= self.dSelectionSpeedGoal {
                    self.dSelectionSpeed = self.dSelectionSpeedGoal;
                    self.iSelectionSpeedTimer = ticks(RANDOM_INT(gp.get(), 60) + 30) as i16;
                }
            }

            if self.dSelectionSpeed <= 0.0f32 {
                self.dSelectionSpeed = 0.0f32;

                if !self.fPowerupSelectionDone {
                    self.fPowerupSelectionDone = true;
                    self.miContinueButton.set_visible(true);
                    self.miContinueButton.select(g, true);

                    //Reset all player's stored item
                    if !g.game_values.keeppowerup {
                        for iPlayer in 0..4usize {
                            g.game_values.storedpowerups[iPlayer] = -1;
                        }
                    }

                    //Give the newly won stored item to the winning players
                    let w = self.iWinningTeam as usize;
                    for iPlayer in 0..g.game_values.teamcounts[w] as usize {
                        gp.get().game_values.storedpowerups[gp.get().game_values.teamids[w][iPlayer] as usize] = self.iSelectedPowerup;
                    }

                    if_sound_on_play(g, &mut gp.rm.sfx_collectpowerup);
                }
            }

            self.dSelectionAngle += self.dSelectionSpeed;

            //If we hit the next powerup, play a tick sound
            if self.dSelectionAngle >= self.dSelectionSector[self.iNextSelectionSoundIndex as usize] {
                if_sound_on_play(g, &mut gp.rm.sfx_worldmove);

                self.iNextSelectionSoundIndex += 1;
                if self.iNextSelectionSoundIndex as i32 > NUMBONUSITEMSONWHEEL {
                    self.iNextSelectionSoundIndex = 1;
                }
            }

            while self.dSelectionAngle > TWO_PI {
                self.dSelectionAngle -= TWO_PI;
            }
        }
    }

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if !self.m_visible {
            return;
        }

        let x = self.m_pos.x as i32;
        let y = self.m_pos.y as i32;

        gp.rm.sprite(gp.rm.spr_tournament_powerup_splash).draw(g, x, y);

        let iSelectorX = (x + 144 + (110.0f32 * cosf(self.dSelectionAngle)) as i16 as i32) as i16;
        let iSelectorY = (y + 190 + (110.0f32 * sinf(self.dSelectionAngle)) as i16 as i32) as i16;

        if self.iState > 0 {
            gp.rm.sprite(gp.rm.spr_powerupselector).draw_src(g, iSelectorX as i32, iSelectorY as i32, &Rect { x: self.iSelectorAnimation as i32 * 64, y: 0, w: 64, h: 64 });
        }

        for iImage in 0..NB {
            if iImage as i16 >= self.iDisplayPowerupIndex {
                break;
            }

            self.miBonusImages[iImage].draw(g);
        }

        for iPlayer in 0..self.iNumPlayers as usize {
            self.miPlayerImages[iPlayer].draw(g);
        }

        self.miContinueButton.draw(g);

        if self.iState == 1 && !self.fPressedSelect {
            gp.rm.menu_font_large.draw_centered(g, App::screenWidth / 2, y + 390, "Press a Button To Stop The Wheel");
        }
    }
}
