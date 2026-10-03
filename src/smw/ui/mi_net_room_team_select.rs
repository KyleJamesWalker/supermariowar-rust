//! Port of src/smw/ui/MI_NetRoomTeamSelect.cpp

use crate::common::global::{game_values, rm, skinlist};
use crate::common::input::CPlayerInput;
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_image::MI_Image;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use sdl2::sys::SDL_Rect;

pub struct MI_NetRoomTeamSelect {
    pub ui_control: UI_Control,

    x: i16,
    y: i16,
    owner_player: i16,

    iAnimationTimer: i16,
    iAnimationFrame: i16,
    iRandomAnimationFrame: i16,

    iFastScroll: i16,
    iFastScrollTimer: i16,

    miModifyImage: Box<MI_Image>,
    miHoverImage: Box<MI_Image>,

    onChangeAccepted: Box<dyn FnMut()>,
}
crate::impl_base!(MI_NetRoomTeamSelect => ui_control: UI_Control);

impl MI_NetRoomTeamSelect {
    pub fn new(x: i16, y: i16, player_id: i16, on_change_accepted: Box<dyn FnMut()>) -> Self {
        unsafe {
            let sel = Ptr::from_mut(&mut rm.menu_player_select);
            let mut miHoverImage = Box::new(MI_Image::new(sel, x - 6, y - 6, 32 + 18, 128 + 18, 44, 44, 1, 1, 1));
            miHoverImage.set_visible(false);
            let mut miModifyImage = Box::new(MI_Image::new(sel, x - 24, y - 24, 32, 128, 78, 78, 4, 1, 8));
            miModifyImage.set_visible(false);

            MI_NetRoomTeamSelect {
                ui_control: UI_Control::new(x, y),
                x,
                y,
                owner_player: player_id,
                iAnimationTimer: 0,
                iAnimationFrame: 0,
                iRandomAnimationFrame: 0,
                iFastScroll: 0,
                iFastScrollTimer: 0,
                miModifyImage,
                miHoverImage,
                onChangeAccepted: on_change_accepted,
            }
        }
    }

    /// `do { step } while (!rm->LoadMenuSkin(...))`
    unsafe fn cycle_skin(&mut self, step: impl Fn(&mut i16)) {
        let p = self.owner_player as usize;
        loop {
            step(&mut game_values.skinids[p]);
            if rm.load_menu_skin(self.owner_player, game_values.skinids[p], game_values.colorids[p], false) {
                break;
            }
        }
    }
}

impl UI_ControlTrait for MI_NetRoomTeamSelect {
    crate::impl_ctl!();

    fn update(&mut self) {
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

        if self.fSelected {
            self.miHoverImage.set_visible(true);
            self.miModifyImage.set_visible(false);

            if self.fModifying {
                self.miHoverImage.set_visible(false);
                self.miModifyImage.set_visible(true);
                self.miModifyImage.update();
            }
        } else {
            self.miHoverImage.set_visible(false);
            self.miModifyImage.set_visible(false);
        }
    }

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        self.miHoverImage.draw();
        self.miModifyImage.draw();

        unsafe {
            let p = self.owner_player as usize;
            let frame = if self.fSelected { self.iAnimationFrame as usize } else { 0 };
            rm.spr_player[p][frame].draw_src(self.x as i32, self.y as i32, &SDL_Rect { x: 0, y: 0, w: 32, h: 32 });
        }
    }

    fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        if playerInput.outputControls[0].menu_select().fPressed || playerInput.outputControls[0].menu_cancel().fPressed {
            (self.onChangeAccepted)();
            self.fModifying = false;
            return MENU_CODE_UNSELECT_ITEM;
        }

        unsafe {
            let playerKeys = game_values.playerInput.outputControls[0];
            let count = skinlist.count() as i16;

            if !game_values.randomskin[0] {
                if playerKeys.menu_up().fPressed {
                    let down = playerKeys.menu_down().fDown;
                    self.cycle_skin(|id| {
                        if down {
                            *id = RANDOM_INT(count as i32) as i16;
                        } else {
                            *id -= 1;
                            if *id < 0 {
                                *id = count - 1;
                            }
                        }
                    });
                } else if playerKeys.menu_up().fDown {
                    if self.iFastScroll == 0 {
                        self.iFastScrollTimer += 1;
                        if self.iFastScrollTimer > 40 {
                            self.iFastScroll = 1;
                        }
                    } else {
                        self.iFastScrollTimer += 1;
                        if self.iFastScrollTimer > 5 {
                            self.cycle_skin(|id| {
                                *id -= 1;
                                if *id < 0 {
                                    *id = count - 1;
                                }
                            });
                            self.iFastScrollTimer = 0;
                        }
                    }
                }

                if playerKeys.menu_down().fPressed {
                    let up = playerKeys.menu_up().fDown;
                    self.cycle_skin(|id| {
                        if up {
                            *id = RANDOM_INT(count as i32) as i16;
                        } else {
                            *id += 1;
                            if *id as i32 >= count as i32 {
                                *id = 0;
                            }
                        }
                    });
                } else if playerKeys.menu_down().fDown {
                    if self.iFastScroll == 0 {
                        self.iFastScrollTimer += 1;
                        if self.iFastScrollTimer > 40 {
                            self.iFastScroll = 1;
                        }
                    } else {
                        self.iFastScrollTimer += 1;
                        if self.iFastScrollTimer > 5 {
                            self.cycle_skin(|id| {
                                *id += 1;
                                if *id as i32 >= count as i32 {
                                    *id = 0;
                                }
                            });
                            self.iFastScrollTimer = 0;
                        }
                    }
                }

                if (!playerKeys.menu_up().fDown && !playerKeys.menu_down().fDown) || (playerKeys.menu_up().fDown && playerKeys.menu_down().fDown) {
                    self.iFastScroll = 0;
                    self.iFastScrollTimer = 0;
                }
            } else {
                self.iFastScroll = 0;
                self.iFastScrollTimer = 0;
            }
        }

        MENU_CODE_NONE
    }

    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        println!("modify");
        self.fModifying = modify;
        MENU_CODE_MODIFY_ACCEPTED
    }
}
