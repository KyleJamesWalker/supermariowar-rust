//! Add, edit and remove entries in the saved server list.
use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::ui::mi_text_field::MI_TextField;
use crate::common::uicontrol::{TextAlign, UI_ControlTrait};
use crate::common::uimenu::UI_Menu;
use crate::globals::Ptr;
use crate::smw::net::{ServerAddress};
use crate::smw::ui::mi_string_scroll::MI_StringScroll;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
/// Which button started the current edit; picking a list entry acts on it.
enum MenuState {
    #[default]
    DEFAULT,
    ADD,
    EDIT,
    DELETE,
}

#[derive(Default)]
/// Add, Edit and Remove buttons beside the saved server list, with an address dialog.
pub struct UI_NetEditServersMenu {
    pub ui_menu: UI_Menu,

    currentState: MenuState,

    miAddButton: Ctl<MI_Button>,
    miEditButton: Ctl<MI_Button>,
    miRemoveButton: Ctl<MI_Button>,
    miBackButton: Ctl<MI_Button>,

    miServerScroll: Ctl<MI_StringScroll>,

    miInstructionsText1: Ctl<MI_Text>,
    miInstructionsText2: Ctl<MI_Text>,

    miDialogTitle: Ctl<MI_Text>,
    miDialogTextField: Ctl<MI_TextField>,
    miDialogOK: Ctl<MI_Button>,
    miDialogCancel: Ctl<MI_Button>,

    /// Backing text of the address field, at most 127 characters.
    dialogTextData: String,

    miLeftHeaderBar: Ctl<MI_Image>,
    miRightHeaderBar: Ctl<MI_Image>,
    miHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_NetEditServersMenu => ui_menu: UI_Menu);

impl UI_NetEditServersMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        this.currentState = MenuState::DEFAULT;
        this.dialogTextData.clear();

        unsafe {
            let selectfield = g.rm.spr_selectfield;
            let plain = g.rm.menu_plain_field;

            this.miBackButton = Ctl::new(MI_Button::new(g, selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
            g.ctl(this.miBackButton).set_code(MENU_CODE_TO_NET_SERVERS_MENU);

            this.miAddButton = Ctl::new(MI_Button::new(g, selectfield, 40, 40, "Add", 200, TextAlign::CENTER), g);
            g.ctl(this.miAddButton).set_code(MENU_CODE_NET_ADDREMOVE_SERVER_ON_ADD_BTN);

            this.miEditButton = Ctl::new(MI_Button::new(g, selectfield, 40, 80, "Edit", 200, TextAlign::CENTER), g);
            g.ctl(this.miEditButton).set_code(MENU_CODE_NET_ADDREMOVE_SERVER_ON_EDIT_BTN);

            this.miRemoveButton = Ctl::new(MI_Button::new(g, selectfield, 40, 120, "Remove", 200, TextAlign::CENTER), g);
            g.ctl(this.miRemoveButton).set_code(MENU_CODE_NET_ADDREMOVE_SERVER_ON_DELETE_BTN);

            this.miInstructionsText1 = Ctl::new(MI_Text::new(g, "", 40, 195, 0, true, TextAlign::LEFT), g);
            this.miInstructionsText2 = Ctl::new(MI_Text::new(g, "", 40, 215, 0, true, TextAlign::LEFT), g);

            this.miLeftHeaderBar = Ctl::new(MI_Image::new(plain, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
            this.miRightHeaderBar = Ctl::new(MI_Image::new(plain, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
            this.miHeaderText = Ctl::new(MI_HeaderText::new(g, "Add/Remove Servers Menu", 320, 5), g);

            this.miServerScroll = Ctl::new(MI_StringScroll::new(plain, 260, 32, 350, 9), g);
            g.ctl(this.miServerScroll).set_auto_modify(true);
            g.ctl(this.miServerScroll).set_accept_code(MENU_CODE_NET_ADDREMOVE_SERVER_ON_SELECT);
            g.ctl(this.miServerScroll).set_cancel_code(MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU);
            g.ctl(this.miServerScroll).deactivate();

            this.miDialogTitle = Ctl::new(MI_Text::new(g, "Enter a new server address below:", 40, 370, 0, true, TextAlign::LEFT), g);
            this.miDialogTextField = Ctl::new(MI_TextField::new(plain, 40, 395, "URL or IP", 640 - 2 * 40, 150), g);
            let data = &mut this.dialogTextData as *mut String;
            g.ctl(this.miDialogTextField).set_data(g, &mut *data, 127);
            this.miDialogOK = Ctl::new(MI_Button::new(g, selectfield, 40, 432, "OK", 100, TextAlign::CENTER), g);
            g.ctl(this.miDialogOK).set_code(MENU_CODE_NET_ADDREMOVE_SERVER_ON_DIALOG_OK_BTN);
            this.miDialogCancel = Ctl::new(MI_Button::new(g, selectfield, 150, 432, "Cancel", 100, TextAlign::CENTER), g);
            g.ctl(this.miDialogCancel).set_code(MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU);
        }

        g.ctl(this.miDialogTitle).set_visible(false);
        g.ctl(this.miDialogTextField).set_visible(false);
        g.ctl(this.miDialogOK).set_visible(false);
        g.ctl(this.miDialogCancel).set_visible(false);

        let add = this.miAddButton.id();
        let edit = this.miEditButton.id();
        let remove = this.miRemoveButton.id();
        let back = this.miBackButton.id();
        let null = ControlId::NONE;

        this.add_control(g, add, back, edit, null, null);
        this.add_control(g, edit, add, remove, null, null);
        this.add_control(g, remove, edit, back, null, null);
        this.add_control(g, back, remove, add, null, null);

        let (l, r, h) = (this.miLeftHeaderBar.id(), this.miRightHeaderBar.id(), this.miHeaderText.id());
        this.add_non_control(l);
        this.add_non_control(r);
        this.add_non_control(h);
        let (i1, i2) = (this.miInstructionsText1.id(), this.miInstructionsText2.id());
        this.add_non_control(i1);
        this.add_non_control(i2);

        let scroll = this.miServerScroll.id();
        this.add_control(g, scroll, null, null, null, null);

        let title = this.miDialogTitle.id();
        this.add_non_control(title);
        let (field, ok, cancel) = (this.miDialogTextField.id(), this.miDialogOK.id(), this.miDialogCancel.id());
        this.add_control(g, field, ok, ok, null, null);
        this.add_control(g, ok, field, field, cancel, cancel);
        this.add_control(g, cancel, field, field, ok, ok);

        this.set_initial_focus(g, add);
        this.set_cancel_code(MENU_CODE_TO_NET_SERVERS_MENU);
        this
    }

    /// Refills the list from `netplay.savedServers`.
    fn reload_scroll(&mut self, g: &mut Game) {
        g.ctl(self.miServerScroll).clear_items();
        for iServer in 0..g.netplay.savedServers.len() {
            let host = g.netplay.savedServers[iServer].hostname.clone();
            g.ctl(self.miServerScroll).add(&host);
        }
    }

    fn show_dialog(&mut self, g: &mut Game) {
        g.ctl(self.miDialogTitle).set_visible(true);
        g.ctl(self.miDialogTextField).set_visible(true);
        g.ctl(self.miDialogOK).set_visible(true);
        g.ctl(self.miDialogCancel).set_visible(true);
    }

    fn hide_dialog(&mut self, g: &mut Game) {
        g.ctl(self.miDialogTitle).set_visible(false);
        g.ctl(self.miDialogTextField).set_visible(false);
        g.ctl(self.miDialogOK).set_visible(false);
        g.ctl(self.miDialogCancel).set_visible(false);
    }

    /// Back to the button column: restores focus, hides the dialog and reloads the list.
    pub fn restore(&mut self, g: &mut Game) {
        if !self.m_savedCurrent.is_none() {
            self.restore_current(g);
        }

        self.set_cancel_code(MENU_CODE_TO_NET_SERVERS_MENU);
        self.currentState = MenuState::DEFAULT;

        g.ctl(self.miInstructionsText1).set_text("");
        g.ctl(self.miInstructionsText2).set_text("");

        self.hide_dialog(g);
        self.reload_scroll(g);
    }

    /// Opens the address dialog for a new server.
    pub fn on_press_add(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.remember_current();
        self.currentState = MenuState::ADD;

        self.show_dialog(g);
        let f = self.miDialogTextField.id();
        self.set_initial_focus(gp.get(), f);
        self.set_cancel_code(MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU);
    }

    /// Focuses the server list to pick the entry to edit; no-op on an empty list.
    pub fn on_press_edit(&mut self, g: &mut Game) {
        if g.netplay.savedServers.is_empty() {
            return;
        }

        self.remember_current();
        self.currentState = MenuState::EDIT;

        let s = self.miServerScroll.id();
        self.set_initial_focus(g, s);
        self.set_cancel_code(MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU);
        g.ctl(self.miServerScroll).activate();

        g.ctl(self.miInstructionsText1).set_text("Select an entry");
        g.ctl(self.miInstructionsText2).set_text("to edit");
    }

    /// Focuses the server list to pick entries to delete; no-op on an empty list.
    pub fn on_press_delete(&mut self, g: &mut Game) {
        if g.netplay.savedServers.is_empty() {
            return;
        }

        self.remember_current();
        self.currentState = MenuState::DELETE;

        let s = self.miServerScroll.id();
        self.set_initial_focus(g, s);
        self.set_cancel_code(MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU);
        g.ctl(self.miServerScroll).activate();

        g.ctl(self.miInstructionsText1).set_text("Select an entry");
        g.ctl(self.miInstructionsText2).set_text("to delete");
    }

    /// Edits or deletes the picked entry, per the current state; deleting stays in the list until it is empty.
    pub fn on_entry_select(&mut self, g: &mut Game) {
        let idx = g.ctl(self.miServerScroll).current_index() as usize;
        match self.currentState {
            MenuState::EDIT => {
                self.dialogTextData = g.netplay.savedServers[idx].hostname.clone();
                g.ctl(self.miDialogTextField).refresh(g);
                self.show_dialog(g);
                let f = self.miDialogTextField.id();
                self.set_initial_focus(g, f);
                self.set_cancel_code(MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU);
            }

            MenuState::DELETE => {
                g.netplay.savedServers.remove(idx);
                self.reload_scroll(g);
                g.ctl(self.miServerScroll).activate();

                if g.netplay.savedServers.is_empty() {
                    self.restore(g);
                }
            }

            _ => {}
        }
    }

    /// Saves the typed address as a new entry or over the edited one, then returns to the buttons.
    pub fn on_dialog_ok(&mut self, g: &mut Game) {
        let new_address = ServerAddress { hostname: self.dialogTextData.clone() };

        match self.currentState {
            MenuState::ADD => g.netplay.savedServers.push(new_address),
            MenuState::EDIT => {
                let idx = g.ctl(self.miServerScroll).current_index() as usize;
                g.netplay.savedServers[idx] = new_address;
            }
            _ => {}
        }

        self.dialogTextData.clear();
        g.ctl(self.miDialogTextField).refresh(g);
        self.restore(g);
    }
}
