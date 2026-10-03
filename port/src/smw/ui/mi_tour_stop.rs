//! Port of src/smw/ui/MI_TourStop.cpp

use crate::common::game_mode::*;
use crate::common::global_constants::NUM_POWERUPS;
use crate::common::match_types::Boss;
use crate::common::ui::menu_code::*;
use crate::common::world_tour_stop::TourStop;
use crate::smw::main::gamemodes;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_image_select_field::MI_ImageSelectField;
use crate::common::ui::mi_map_field::MI_MapField;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{TextAlign, UI_Control, UI_ControlTrait};
use crate::globals::*;
use crate::smw::main::{bossgamemode, boxesgamemode, pipegamemode};

pub struct MI_TourStop {
    pub ui_control: UI_Control,

    miModeField: Box<MI_ImageSelectField>,
    miGoalField: Box<MI_SelectField<i16>>,
    miPointsField: Box<MI_SelectField<i16>>,
    miMapField: Box<MI_MapField>,
    miStartButton: Box<MI_Button>,

    miBonusField: Option<Box<MI_SelectField<i16>>>,
    miEndStageImage: [Option<Box<MI_Image>>; 2],

    miBonusIcon: [Option<Box<MI_Image>>; 10],
    miBonusBackground: [Option<Box<MI_Image>>; 10],

    miTourStopLeftHeaderBar: Box<MI_Image>,
    miTourStopMenuRightHeaderBar: Box<MI_Image>,
    miTourStopMenuHeaderText: Box<MI_Text>,

    fIsWorld: bool,
}
crate::impl_base!(MI_TourStop => ui_control: UI_Control);

impl MI_TourStop {
    //Call with x = 70 and y == 80
    pub fn new(x: i16, y: i16, fWorld: bool) -> Self {
        unsafe {
            let fIsWorld = fWorld;
            let spr_disabled = Ptr::from_mut(&mut rm.spr_selectfielddisabled);

            let mut miModeField;
            let mut miGoalField;
            let mut miPointsField;
            let miBonusField;
            let mut miEndStageImage: [Option<Box<MI_Image>>; 2] = [None, None];
            let mut miBonusIcon: [Option<Box<MI_Image>>; 10] = Default::default();
            let mut miBonusBackground: [Option<Box<MI_Image>>; 10] = Default::default();

            if fIsWorld {
                miModeField = Box::new(MI_ImageSelectField::new(spr_disabled, Ptr::from_mut(&mut rm.menu_mode_small), 70, 85, "Mode", 305, 90, 16, 16));
                miGoalField = Box::new(MI_SelectField::<i16>::new(spr_disabled, 380, 85, "Goal", 190, 90));
                miPointsField = Box::new(MI_SelectField::<i16>::new(spr_disabled, 380, 125, "Score", 190, 90));

                let mut bonus = Box::new(MI_SelectField::<i16>::new(spr_disabled, 70, 125, "Bonus", 305, 90));
                bonus.disable(true);
                miBonusField = Some(bonus);

                let mut img0 = Box::new(MI_Image::new(Ptr::from_mut(&mut rm.spr_worlditemsplace), 54, 201, 0, 20, 80, 248, 1, 1, 0));
                img0.set_visible(false);
                miEndStageImage[0] = Some(img0);

                let mut img1 = Box::new(MI_Image::new(Ptr::from_mut(&mut rm.spr_worlditemsplace), 506, 201, 0, 20, 80, 248, 1, 1, 0));
                img1.set_visible(false);
                miEndStageImage[1] = Some(img1);

                for iBonus in 0..10i16 {
                    let mut icon = Box::new(MI_Image::new(Ptr::from_mut(&mut rm.spr_worlditemssmall), 170 + iBonus * 20, 133, 0, 0, 16, 16, 1, 1, 0));
                    let mut background = Box::new(MI_Image::new(Ptr::from_mut(&mut rm.spr_worlditemsplace), 168 + iBonus * 20, 131, 0, 0, 20, 20, 1, 1, 0));

                    icon.set_visible(false);
                    background.set_visible(false);

                    miBonusIcon[iBonus as usize] = Some(icon);
                    miBonusBackground[iBonus as usize] = Some(background);
                }
            } else {
                miModeField = Box::new(MI_ImageSelectField::new(spr_disabled, Ptr::from_mut(&mut rm.menu_mode_small), 70, 85, "Mode", 500, 120, 16, 16));
                miGoalField = Box::new(MI_SelectField::<i16>::new(spr_disabled, 70, 125, "Goal", 246, 120));
                miPointsField = Box::new(MI_SelectField::<i16>::new(spr_disabled, 70 + 254, 125, "Score", 246, 120));

                miBonusField = None;
            }

            let mut miStartButton = Box::new(MI_Button::new(Ptr::from_mut(&mut rm.spr_selectfield), 70, 45, "Start", 500, TextAlign::LEFT));
            miStartButton.set_code(MENU_CODE_TOUR_STOP_CONTINUE);
            miStartButton.select(true);

            let mut miMapField = Box::new(MI_MapField::new(spr_disabled, 70, 165, "Map", 500, 120, false));
            miMapField.disable(true);

            miModeField.disable(true);
            miGoalField.disable(true);
            miPointsField.disable(true);

            let miTourStopLeftHeaderBar = Box::new(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 0, 0, 0, 0, 320, 32, 1, 1, 0));
            let miTourStopMenuRightHeaderBar = Box::new(MI_Image::new(Ptr::from_mut(&mut rm.menu_plain_field), 320, 0, 192, 0, 320, 32, 1, 1, 0));
            let miTourStopMenuHeaderText = Box::new(MI_HeaderText::new("Tour Stop", 320, 5));

            MI_TourStop {
                ui_control: UI_Control::new(x, y),
                miModeField,
                miGoalField,
                miPointsField,
                miMapField,
                miStartButton,
                miBonusField,
                miEndStageImage,
                miBonusIcon,
                miBonusBackground,
                miTourStopLeftHeaderBar,
                miTourStopMenuRightHeaderBar,
                miTourStopMenuHeaderText,
                fIsWorld,
            }
        }
    }

    /// `MI_TourStop::Refresh(short iTourStop)`; hides the `UI_Control::Refresh()` virtual like the C++ overload.
    pub fn refresh(&mut self, iTourStop: i16) {
        unsafe {
            let tourstop: Ptr<TourStop> = game_values.tourstops[iTourStop as usize];

            if tourstop.iStageType == 0 {
                self.miModeField.clear();

                let gamemode: Ptr<dyn CGameModeTrait>;
                let tourstopicon: i16;
                if tourstop.iMode as i32 == game_mode_pipe_minigame {
                    gamemode = Ptr::from_raw(pipegamemode.as_ptr() as *mut dyn CGameModeTrait);
                    tourstopicon = 25;
                } else if tourstop.iMode as i32 == game_mode_boss_minigame {
                    bossgamemode.set_boss_type(tourstop.gmsSettings.boss.bosstype);
                    gamemode = Ptr::from_raw(bossgamemode.as_ptr() as *mut dyn CGameModeTrait);
                    tourstopicon = 26;
                } else if tourstop.iMode as i32 == game_mode_boxes_minigame {
                    gamemode = Ptr::from_raw(boxesgamemode.as_ptr() as *mut dyn CGameModeTrait);
                    tourstopicon = 27;
                } else {
                    gamemode = gamemodes[tourstop.iMode as usize];
                    tourstopicon = tourstop.iMode;
                }

                let szModeName = gamemode.get_mode_name().to_string();
                let szGoalName = gamemode.get_goal_name().to_string();

                self.miModeField.add(szModeName, tourstopicon);

                self.miGoalField.clear();
                let szTemp = format!("{}", tourstop.iGoal);
                self.miGoalField.add_random(szTemp, 0, false);
                self.miGoalField.set_title(szGoalName);

                self.miPointsField.clear();
                let szTemp = format!("{}", tourstop.iPoints);
                self.miPointsField.add_random(szTemp, 0, false);

                if tourstop.iMode as i32 == game_mode_pipe_minigame {
                    let fFound = self.miMapField.set_map(&tourstop.pszMapFile, true);

                    if !fFound {
                        self.miMapField.set_special_map("Pipe Minigame", "maps/special/two52_special_pipe_minigame.map");
                    }
                } else if tourstop.iMode as i32 == game_mode_boss_minigame {
                    let fFound = self.miMapField.set_map(&tourstop.pszMapFile, true);

                    if !fFound {
                        match tourstop.gmsSettings.boss.bosstype {
                            Boss::Hammer => {
                                self.miMapField.set_special_map("Hammer Boss Minigame", "maps/special/two52_special_hammerboss_minigame.map");
                            }
                            Boss::Bomb => {
                                self.miMapField.set_special_map("Bomb Boss Minigame", "maps/special/two52_special_bombboss_minigame.map");
                            }
                            Boss::Fire => {
                                self.miMapField.set_special_map("Fire Boss Minigame", "maps/special/two52_special_fireboss_minigame.map");
                            }
                        }
                    }
                } else if tourstop.iMode as i32 == game_mode_boxes_minigame {
                    let fFound = self.miMapField.set_map(&tourstop.pszMapFile, true);

                    if !fFound {
                        self.miMapField.set_special_map("Boxes Minigame", "maps/special/two52_special_boxes_minigame.map");
                    }
                } else {
                    self.miMapField.set_map(&tourstop.pszMapFile, true);
                }

                self.miTourStopMenuHeaderText.set_text(tourstop.szName.clone());

                if self.fIsWorld {
                    self.miBonusField.as_mut().unwrap().clear();
                    self.miEndStageImage[0].as_mut().unwrap().set_visible(tourstop.fEndStage);
                    self.miEndStageImage[1].as_mut().unwrap().set_visible(tourstop.fEndStage);

                    for iBonus in 0..10i16 {
                        let fShowBonus = iBonus < tourstop.iNumBonuses;
                        let icon = self.miBonusIcon[iBonus as usize].as_mut().unwrap();
                        let background = self.miBonusBackground[iBonus as usize].as_mut().unwrap();
                        if fShowBonus {
                            let iBonusIcon = tourstop.wsbBonuses[iBonus as usize].iBonus;
                            icon.set_image_source(if (iBonusIcon as i32) < NUM_POWERUPS {
                                Ptr::from_mut(&mut rm.spr_storedpowerupsmall)
                            } else {
                                Ptr::from_mut(&mut rm.spr_worlditemssmall)
                            });
                            icon.set_image(
                                ((if (iBonusIcon as i32) < NUM_POWERUPS { iBonusIcon as i32 } else { iBonusIcon as i32 - NUM_POWERUPS }) << 4) as i16,
                                0,
                                16,
                                16,
                            );
                            background.set_image((tourstop.wsbBonuses[iBonus as usize].iWinnerPlace as i32 * 20) as i16, 0, 20, 20);
                        }

                        icon.set_visible(fShowBonus);
                        background.set_visible(fShowBonus);
                    }
                }
            }
        }
    }
}

impl UI_ControlTrait for MI_TourStop {
    crate::impl_ctl!();

    fn modify(&mut self, fModify: bool) -> MenuCodeEnum {
        self.miStartButton.modify(fModify)
    }

    fn update(&mut self) {
        if !self.m_visible {
            return;
        }

        self.miStartButton.update();

        self.miModeField.update();
        self.miGoalField.update();
        self.miPointsField.update();
        self.miMapField.update();

        if self.fIsWorld {
            self.miBonusField.as_mut().unwrap().update();
        }

        self.miTourStopLeftHeaderBar.update();
        self.miTourStopMenuRightHeaderBar.update();
        self.miTourStopMenuHeaderText.update();
    }

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        self.miStartButton.draw();

        self.miModeField.draw();
        self.miGoalField.draw();
        self.miPointsField.draw();
        self.miMapField.draw();

        if self.fIsWorld {
            self.miBonusField.as_mut().unwrap().draw();
            self.miEndStageImage[0].as_mut().unwrap().draw();
            self.miEndStageImage[1].as_mut().unwrap().draw();

            for iBonus in 0..10 {
                self.miBonusBackground[iBonus].as_mut().unwrap().draw();
                self.miBonusIcon[iBonus].as_mut().unwrap().draw();
            }
        }

        self.miTourStopLeftHeaderBar.draw();
        self.miTourStopMenuRightHeaderBar.draw();
        self.miTourStopMenuHeaderText.draw();
    }
}
