//! Port of src/smw/menu/network/NetEditServersMenu.cpp

use crate::common::global::rm;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::ui::mi_text_field::MI_TextField;
use crate::common::uicontrol::{ctl_ptr, TextAlign, UI_ControlTrait};
use crate::common::uimenu::UI_Menu;
use crate::globals::Ptr;
use crate::smw::net::{netplay, ServerAddress};
use crate::smw::ui::mi_string_scroll::MI_StringScroll;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum MenuState {
    #[default]
    DEFAULT,
    ADD,
    EDIT,
    DELETE,
}

#[derive(Default)]
pub struct UI_NetEditServersMenu {
    pub ui_menu: UI_Menu,

    currentState: MenuState,

    miAddButton: Ptr<MI_Button>,
    miEditButton: Ptr<MI_Button>,
    miRemoveButton: Ptr<MI_Button>,
    miBackButton: Ptr<MI_Button>,

    miServerScroll: Ptr<MI_StringScroll>,

    miInstructionsText1: Ptr<MI_Text>,
    miInstructionsText2: Ptr<MI_Text>,

    miDialogTitle: Ptr<MI_Text>,
    miDialogTextField: Ptr<MI_TextField>,
    miDialogOK: Ptr<MI_Button>,
    miDialogCancel: Ptr<MI_Button>,

    dialogTextData: String,

    miLeftHeaderBar: Ptr<MI_Image>,
    miRightHeaderBar: Ptr<MI_Image>,
    miHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_NetEditServersMenu => ui_menu: UI_Menu);

impl UI_NetEditServersMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        this.currentState = MenuState::DEFAULT;
        this.dialogTextData.clear();

        unsafe {
            let selectfield = Ptr::from_mut(&mut rm.spr_selectfield);
            let plain = Ptr::from_mut(&mut rm.menu_plain_field);

            this.miBackButton = Ptr::new_box(MI_Button::new(selectfield, 544, 432, "Back", 80, TextAlign::CENTER));
            this.miBackButton.set_code(MENU_CODE_TO_NET_SERVERS_MENU);

            this.miAddButton = Ptr::new_box(MI_Button::new(selectfield, 40, 40, "Add", 200, TextAlign::CENTER));
            this.miAddButton.set_code(MENU_CODE_NET_ADDREMOVE_SERVER_ON_ADD_BTN);

            this.miEditButton = Ptr::new_box(MI_Button::new(selectfield, 40, 80, "Edit", 200, TextAlign::CENTER));
            this.miEditButton.set_code(MENU_CODE_NET_ADDREMOVE_SERVER_ON_EDIT_BTN);

            this.miRemoveButton = Ptr::new_box(MI_Button::new(selectfield, 40, 120, "Remove", 200, TextAlign::CENTER));
            this.miRemoveButton.set_code(MENU_CODE_NET_ADDREMOVE_SERVER_ON_DELETE_BTN);

            this.miInstructionsText1 = Ptr::new_box(MI_Text::new("", 40, 195, 0, true, TextAlign::LEFT));
            this.miInstructionsText2 = Ptr::new_box(MI_Text::new("", 40, 215, 0, true, TextAlign::LEFT));

            this.miLeftHeaderBar = Ptr::new_box(MI_Image::new(plain, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miRightHeaderBar = Ptr::new_box(MI_Image::new(plain, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miHeaderText = Ptr::new_box(MI_HeaderText::new("Add/Remove Servers Menu", 320, 5));

            this.miServerScroll = Ptr::new_box(MI_StringScroll::new(plain, 260, 32, 350, 9));
            this.miServerScroll.set_auto_modify(true);
            this.miServerScroll.set_accept_code(MENU_CODE_NET_ADDREMOVE_SERVER_ON_SELECT);
            this.miServerScroll.set_cancel_code(MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU);
            this.miServerScroll.deactivate();

            this.miDialogTitle = Ptr::new_box(MI_Text::new("Enter a new server address below:", 40, 370, 0, true, TextAlign::LEFT));
            this.miDialogTextField = Ptr::new_box(MI_TextField::new(plain, 40, 395, "URL or IP", 640 - 2 * 40, 150));
            let data = &mut this.dialogTextData as *mut String;
            this.miDialogTextField.set_data(&mut *data, 127);
            this.miDialogOK = Ptr::new_box(MI_Button::new(selectfield, 40, 432, "OK", 100, TextAlign::CENTER));
            this.miDialogOK.set_code(MENU_CODE_NET_ADDREMOVE_SERVER_ON_DIALOG_OK_BTN);
            this.miDialogCancel = Ptr::new_box(MI_Button::new(selectfield, 150, 432, "Cancel", 100, TextAlign::CENTER));
            this.miDialogCancel.set_code(MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU);
        }

        this.miDialogTitle.set_visible(false);
        this.miDialogTextField.set_visible(false);
        this.miDialogOK.set_visible(false);
        this.miDialogCancel.set_visible(false);

        let add = ctl_ptr(this.miAddButton);
        let edit = ctl_ptr(this.miEditButton);
        let remove = ctl_ptr(this.miRemoveButton);
        let back = ctl_ptr(this.miBackButton);
        let null = Ptr::null();

        this.add_control(add, back, edit, null, null);
        this.add_control(edit, add, remove, null, null);
        this.add_control(remove, edit, back, null, null);
        this.add_control(back, remove, add, null, null);

        let (l, r, h) = (ctl_ptr(this.miLeftHeaderBar), ctl_ptr(this.miRightHeaderBar), ctl_ptr(this.miHeaderText));
        this.add_non_control(l);
        this.add_non_control(r);
        this.add_non_control(h);
        let (i1, i2) = (ctl_ptr(this.miInstructionsText1), ctl_ptr(this.miInstructionsText2));
        this.add_non_control(i1);
        this.add_non_control(i2);

        let scroll = ctl_ptr(this.miServerScroll);
        this.add_control(scroll, null, null, null, null);

        let title = ctl_ptr(this.miDialogTitle);
        this.add_non_control(title);
        let (field, ok, cancel) = (ctl_ptr(this.miDialogTextField), ctl_ptr(this.miDialogOK), ctl_ptr(this.miDialogCancel));
        this.add_control(field, ok, ok, null, null);
        this.add_control(ok, field, field, cancel, cancel);
        this.add_control(cancel, field, field, ok, ok);

        this.set_initial_focus(add);
        this.set_cancel_code(MENU_CODE_TO_NET_SERVERS_MENU);
        this
    }

    fn reload_scroll(&mut self) {
        self.miServerScroll.clear_items();
        unsafe {
            for iServer in 0..netplay.savedServers.len() {
                let host = netplay.savedServers[iServer].hostname.clone();
                self.miServerScroll.add(&host);
            }
        }
    }

    fn show_dialog(&mut self) {
        self.miDialogTitle.set_visible(true);
        self.miDialogTextField.set_visible(true);
        self.miDialogOK.set_visible(true);
        self.miDialogCancel.set_visible(true);
    }

    fn hide_dialog(&mut self) {
        self.miDialogTitle.set_visible(false);
        self.miDialogTextField.set_visible(false);
        self.miDialogOK.set_visible(false);
        self.miDialogCancel.set_visible(false);
    }

    pub fn restore(&mut self) {
        if !self.m_savedCurrent.is_null() {
            self.restore_current();
        }

        self.set_cancel_code(MENU_CODE_TO_NET_SERVERS_MENU);
        self.currentState = MenuState::DEFAULT;

        self.miInstructionsText1.set_text("");
        self.miInstructionsText2.set_text("");

        self.hide_dialog();
        self.reload_scroll();
    }

    pub fn on_press_add(&mut self) {
        self.remember_current();
        self.currentState = MenuState::ADD;

        self.show_dialog();
        let f = ctl_ptr(self.miDialogTextField);
        self.set_initial_focus(f);
        self.set_cancel_code(MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU);
    }

    pub fn on_press_edit(&mut self) {
        if unsafe { netplay.savedServers.is_empty() } {
            return;
        }

        self.remember_current();
        self.currentState = MenuState::EDIT;

        let s = ctl_ptr(self.miServerScroll);
        self.set_initial_focus(s);
        self.set_cancel_code(MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU);
        self.miServerScroll.activate();

        self.miInstructionsText1.set_text("Select an entry");
        self.miInstructionsText2.set_text("to edit");
    }

    pub fn on_press_delete(&mut self) {
        if unsafe { netplay.savedServers.is_empty() } {
            return;
        }

        self.remember_current();
        self.currentState = MenuState::DELETE;

        let s = ctl_ptr(self.miServerScroll);
        self.set_initial_focus(s);
        self.set_cancel_code(MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU);
        self.miServerScroll.activate();

        self.miInstructionsText1.set_text("Select an entry");
        self.miInstructionsText2.set_text("to delete");
    }

    pub fn on_entry_select(&mut self) {
        let idx = self.miServerScroll.current_index() as usize;
        unsafe {
            match self.currentState {
                MenuState::EDIT => {
                    self.dialogTextData = netplay.savedServers[idx].hostname.clone();
                    self.miDialogTextField.refresh();
                    self.show_dialog();
                    let f = ctl_ptr(self.miDialogTextField);
                    self.set_initial_focus(f);
                    self.set_cancel_code(MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU);
                }

                MenuState::DELETE => {
                    netplay.savedServers.remove(idx);
                    self.reload_scroll();
                    self.miServerScroll.activate();

                    if netplay.savedServers.is_empty() {
                        self.restore();
                    }
                }

                _ => {}
            }
        }
    }

    pub fn on_dialog_ok(&mut self) {
        let new_address = ServerAddress { hostname: self.dialogTextData.clone() };

        unsafe {
            match self.currentState {
                MenuState::ADD => netplay.savedServers.push(new_address),
                MenuState::EDIT => {
                    let idx = self.miServerScroll.current_index() as usize;
                    netplay.savedServers[idx] = new_address;
                }
                _ => {}
            }
        }

        self.dialogTextData.clear();
        self.miDialogTextField.refresh();
        self.restore();
    }
}
