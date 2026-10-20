//! The tour stop screen shown before each stop of a tour, with an Exit Tour dialog.

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_tour_stop::MI_TourStop;

#[derive(Default)]
pub struct UI_TourStopMenu {
    pub ui_menu: UI_Menu,

    pub miTourStop: Ctl<MI_TourStop>,

    pub miTourStopExitDialogImage: Ctl<MI_Image>,
    pub miTourStopExitDialogExitTourText: Ctl<MI_Text>,
    pub miTourStopExitDialogYesButton: Ctl<MI_Button>,
    pub miTourStopExitDialogNoButton: Ctl<MI_Button>,
}
crate::impl_base!(UI_TourStopMenu => ui_menu: UI_Menu);

impl UI_TourStopMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        let spr_selectfield = g.rm.spr_selectfield;

        this.miTourStop = Ctl::new(MI_TourStop::new(g, 70, 45, false), g);

        // Exit tour dialog box
        this.miTourStopExitDialogImage = Ctl::new(MI_Image::new(g.rm.spr_dialog, 224, 176, 0, 0, 192, 128, 1, 1, 0), g);
        this.miTourStopExitDialogExitTourText = Ctl::new(MI_HeaderText::new(g, "Exit Tour", 320, 205), g);

        this.miTourStopExitDialogYesButton = Ctl::new(MI_Button::new(g, spr_selectfield, 235, 250, "Yes", 80, TextAlign::CENTER), g);
        this.miTourStopExitDialogNoButton = Ctl::new(MI_Button::new(g, spr_selectfield, 325, 250, "No", 80, TextAlign::CENTER), g);

        g.ctl(this.miTourStopExitDialogYesButton).set_code(MENU_CODE_EXIT_TOUR_YES);
        g.ctl(this.miTourStopExitDialogNoButton).set_code(MENU_CODE_EXIT_TOUR_NO);

        g.ctl(this.miTourStopExitDialogImage).set_visible(false);
        g.ctl(this.miTourStopExitDialogExitTourText).set_visible(false);
        g.ctl(this.miTourStopExitDialogYesButton).set_visible(false);
        g.ctl(this.miTourStopExitDialogNoButton).set_visible(false);

        let stop = this.miTourStop.id();
        let img = this.miTourStopExitDialogImage.id();
        let text = this.miTourStopExitDialogExitTourText.id();
        let yes = this.miTourStopExitDialogYesButton.id();
        let no = this.miTourStopExitDialogNoButton.id();

        this.add_control(g, stop, ControlId::NONE, ControlId::NONE, ControlId::NONE, ControlId::NONE);

        this.add_non_control(img);
        this.add_non_control(text);

        this.add_control(g, yes, ControlId::NONE, ControlId::NONE, ControlId::NONE, no);
        this.add_control(g, no, ControlId::NONE, ControlId::NONE, yes, ControlId::NONE);

        this.set_initial_focus(g, stop);
        this.set_cancel_code(MENU_CODE_BACK_TEAM_SELECT_MENU);
        this
    }

    /// Opens the Exit Tour dialog with No focused and cancel disabled.
    pub fn open_exit_dialog(&mut self, g: &mut Game) {
        g.ctl(self.miTourStopExitDialogImage).set_visible(true);
        g.ctl(self.miTourStopExitDialogExitTourText).set_visible(true);
        g.ctl(self.miTourStopExitDialogYesButton).set_visible(true);
        g.ctl(self.miTourStopExitDialogNoButton).set_visible(true);

        self.remember_current();

        let focus = self.miTourStopExitDialogNoButton.id();
        self.set_initial_focus(g, focus);
        self.set_cancel_code(MENU_CODE_NONE);
        self.reset_menu(g);
    }

    /// Hides the Exit Tour dialog and restores the remembered focus.
    pub fn close_exit_dialog(&mut self, g: &mut Game) {
        g.ctl(self.miTourStopExitDialogImage).set_visible(false);
        g.ctl(self.miTourStopExitDialogExitTourText).set_visible(false);
        g.ctl(self.miTourStopExitDialogYesButton).set_visible(false);
        g.ctl(self.miTourStopExitDialogNoButton).set_visible(false);

        let focus = self.miTourStop.id();
        self.set_initial_focus(g, focus);
        self.set_cancel_code(MENU_CODE_BACK_TEAM_SELECT_MENU);

        self.restore_current(g);
    }
}
