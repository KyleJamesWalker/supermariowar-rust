//! Port of src/smw/menu/TourStopMenu.cpp

use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_tour_stop::MI_TourStop;

#[derive(Default)]
pub struct UI_TourStopMenu {
    pub ui_menu: UI_Menu,

    pub miTourStop: Ptr<MI_TourStop>,

    pub miTourStopExitDialogImage: Ptr<MI_Image>,
    pub miTourStopExitDialogExitTourText: Ptr<MI_Text>,
    pub miTourStopExitDialogYesButton: Ptr<MI_Button>,
    pub miTourStopExitDialogNoButton: Ptr<MI_Button>,
}
crate::impl_base!(UI_TourStopMenu => ui_menu: UI_Menu);

impl UI_TourStopMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr_selectfield = Ptr::from_mut(&mut rm.spr_selectfield);

            this.miTourStop = Ptr::new_box(MI_TourStop::new(70, 45, false));

            // Exit tour dialog box
            this.miTourStopExitDialogImage = Ptr::new_box(MI_Image::new(Ptr::from_mut(&mut rm.spr_dialog), 224, 176, 0, 0, 192, 128, 1, 1, 0));
            this.miTourStopExitDialogExitTourText = Ptr::new_box(MI_HeaderText::new("Exit Tour", 320, 205));

            this.miTourStopExitDialogYesButton = Ptr::new_box(MI_Button::new(spr_selectfield, 235, 250, "Yes", 80, TextAlign::CENTER));
            this.miTourStopExitDialogNoButton = Ptr::new_box(MI_Button::new(spr_selectfield, 325, 250, "No", 80, TextAlign::CENTER));
        }

        this.miTourStopExitDialogYesButton.set_code(MENU_CODE_EXIT_TOUR_YES);
        this.miTourStopExitDialogNoButton.set_code(MENU_CODE_EXIT_TOUR_NO);

        this.miTourStopExitDialogImage.set_visible(false);
        this.miTourStopExitDialogExitTourText.set_visible(false);
        this.miTourStopExitDialogYesButton.set_visible(false);
        this.miTourStopExitDialogNoButton.set_visible(false);

        let stop = ctl_ptr(this.miTourStop);
        let img = ctl_ptr(this.miTourStopExitDialogImage);
        let text = ctl_ptr(this.miTourStopExitDialogExitTourText);
        let yes = ctl_ptr(this.miTourStopExitDialogYesButton);
        let no = ctl_ptr(this.miTourStopExitDialogNoButton);

        this.add_control(stop, Ptr::null(), Ptr::null(), Ptr::null(), Ptr::null());

        this.add_non_control(img);
        this.add_non_control(text);

        this.add_control(yes, Ptr::null(), Ptr::null(), Ptr::null(), no);
        this.add_control(no, Ptr::null(), Ptr::null(), yes, Ptr::null());

        this.set_initial_focus(stop);
        this.set_cancel_code(MENU_CODE_BACK_TEAM_SELECT_MENU);
        this
    }

    pub fn open_exit_dialog(&mut self) {
        self.miTourStopExitDialogImage.set_visible(true);
        self.miTourStopExitDialogExitTourText.set_visible(true);
        self.miTourStopExitDialogYesButton.set_visible(true);
        self.miTourStopExitDialogNoButton.set_visible(true);

        self.remember_current();

        let focus = ctl_ptr(self.miTourStopExitDialogNoButton);
        self.set_initial_focus(focus);
        self.set_cancel_code(MENU_CODE_NONE);
        self.reset_menu();
    }

    pub fn close_exit_dialog(&mut self) {
        self.miTourStopExitDialogImage.set_visible(false);
        self.miTourStopExitDialogExitTourText.set_visible(false);
        self.miTourStopExitDialogYesButton.set_visible(false);
        self.miTourStopExitDialogNoButton.set_visible(false);

        let focus = ctl_ptr(self.miTourStop);
        self.set_initial_focus(focus);
        self.set_cancel_code(MENU_CODE_BACK_TEAM_SELECT_MENU);

        self.restore_current();
    }
}
