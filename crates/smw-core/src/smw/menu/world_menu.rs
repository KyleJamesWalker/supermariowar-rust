//! The world map screen, with its stage start popup and Exit World dialog.

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
use crate::smw::ui::mi_world::MI_World;

/// The world map with its stage popup; cancel opens the Exit World dialog.
#[derive(Default)]
pub struct UI_WorldMenu {
    pub ui_menu: UI_Menu,

    pub miWorld: Ctl<MI_World>,
    /// Stage start popup for the stage a team entered, hidden until [`Self::open_stage_start`].
    pub miWorldStop: Ctl<MI_TourStop>,

    pub miWorldExitDialogImage: Ctl<MI_Image>,
    pub miWorldExitDialogExitTourText: Ctl<MI_Text>,
    pub miWorldExitDialogYesButton: Ctl<MI_Button>,
    pub miWorldExitDialogNoButton: Ctl<MI_Button>,
}
crate::impl_base!(UI_WorldMenu => ui_menu: UI_Menu);

impl UI_WorldMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        let spr_selectfield = g.rm.spr_selectfield;

        this.miWorld = Ctl::new(MI_World::new(g), g);
        g.ctl(this.miWorld).set_auto_modify(true);

        this.miWorldStop = Ctl::new(MI_TourStop::new(g, 70, 45, true), g);
        g.ctl(this.miWorldStop).set_visible(false);

        // Exit tour dialog box
        this.miWorldExitDialogImage = Ctl::new(MI_Image::new(g.rm.spr_dialog, 224, 176, 0, 0, 192, 128, 1, 1, 0), g);
        this.miWorldExitDialogExitTourText = Ctl::new(MI_HeaderText::new(g, "Exit World", 320, 205), g);

        this.miWorldExitDialogYesButton = Ctl::new(MI_Button::new(g, spr_selectfield, 235, 250, "Yes", 80, TextAlign::CENTER), g);
        this.miWorldExitDialogNoButton = Ctl::new(MI_Button::new(g, spr_selectfield, 325, 250, "No", 80, TextAlign::CENTER), g);

        g.ctl(this.miWorldExitDialogYesButton).set_code(MENU_CODE_EXIT_WORLD_YES);
        g.ctl(this.miWorldExitDialogNoButton).set_code(MENU_CODE_EXIT_WORLD_NO);

        g.ctl(this.miWorldExitDialogImage).set_visible(false);
        g.ctl(this.miWorldExitDialogExitTourText).set_visible(false);
        g.ctl(this.miWorldExitDialogYesButton).set_visible(false);
        g.ctl(this.miWorldExitDialogNoButton).set_visible(false);

        let world = this.miWorld.id();
        let stop = this.miWorldStop.id();
        let img = this.miWorldExitDialogImage.id();
        let text = this.miWorldExitDialogExitTourText.id();
        let yes = this.miWorldExitDialogYesButton.id();
        let no = this.miWorldExitDialogNoButton.id();

        this.add_control(g, world, ControlId::NONE, ControlId::NONE, ControlId::NONE, ControlId::NONE);

        this.add_control(g, stop, ControlId::NONE, ControlId::NONE, ControlId::NONE, ControlId::NONE);

        this.add_non_control(img);
        this.add_non_control(text);

        this.add_control(g, yes, ControlId::NONE, ControlId::NONE, ControlId::NONE, no);
        this.add_control(g, no, ControlId::NONE, ControlId::NONE, yes, ControlId::NONE);

        this.set_initial_focus(g, world);
        this.set_cancel_code(MENU_CODE_BACK_TEAM_SELECT_MENU);
        this
    }

    /// Shows the stage popup for `game_values.tourstopcurrent` and focuses it.
    pub fn open_stage_start(&mut self, g: &mut Game) {
        g.ctl(self.miWorldStop).refresh(g, g.game_values.tourstopcurrent as i16);
        g.ctl(self.miWorldStop).set_visible(true);

        self.remember_current();

        let focus = self.miWorldStop.id();
        self.set_initial_focus(g, focus);
        self.set_cancel_code(MENU_CODE_WORLD_STAGE_NO_START);

        self.reset_menu(g);
    }

    /// Hides the stage popup and restores the remembered focus.
    pub fn close_stage_start(&mut self, g: &mut Game) {
        g.ctl(self.miWorldStop).set_visible(false);

        let focus = self.miWorld.id();
        self.set_initial_focus(g, focus);
        self.set_cancel_code(MENU_CODE_BACK_TEAM_SELECT_MENU);

        self.restore_current(g);
    }

    /// Opens the Exit World dialog with No focused and cancel disabled.
    pub fn open_exit_dialog(&mut self, g: &mut Game) {
        g.ctl(self.miWorldExitDialogImage).set_visible(true);
        g.ctl(self.miWorldExitDialogExitTourText).set_visible(true);
        g.ctl(self.miWorldExitDialogYesButton).set_visible(true);
        g.ctl(self.miWorldExitDialogNoButton).set_visible(true);

        self.remember_current();

        let focus = self.miWorldExitDialogNoButton.id();
        self.set_initial_focus(g, focus);
        self.set_cancel_code(MENU_CODE_NONE);
        self.reset_menu(g);
    }

    /// Hides the Exit World dialog and restores the remembered focus.
    pub fn close_exit_dialog(&mut self, g: &mut Game) {
        g.ctl(self.miWorldExitDialogImage).set_visible(false);
        g.ctl(self.miWorldExitDialogExitTourText).set_visible(false);
        g.ctl(self.miWorldExitDialogYesButton).set_visible(false);
        g.ctl(self.miWorldExitDialogNoButton).set_visible(false);

        let focus = self.miWorld.id();
        self.set_initial_focus(g, focus);
        self.set_cancel_code(MENU_CODE_BACK_TEAM_SELECT_MENU);

        self.restore_current(g);
    }
}
