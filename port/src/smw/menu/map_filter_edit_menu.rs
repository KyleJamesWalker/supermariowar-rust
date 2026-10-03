//! Port of src/smw/menu/MapFilterEditMenu.cpp

use crate::common::ui::menu_code::*;
use crate::common::uicontrol::ctl_ptr;
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::ui::mi_map_browser::MI_MapBrowser;

#[derive(Default)]
pub struct UI_MapFilterEditMenu {
    pub ui_menu: UI_Menu,

    pub miMapBrowser: Ptr<MI_MapBrowser>,
}
crate::impl_base!(UI_MapFilterEditMenu => ui_menu: UI_Menu);

impl UI_MapFilterEditMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();

        this.miMapBrowser = Ptr::new_box(MI_MapBrowser::new());
        this.miMapBrowser.set_auto_modify(true);

        let b = ctl_ptr(this.miMapBrowser);
        this.add_control(b, Ptr::null(), Ptr::null(), Ptr::null(), Ptr::null());
        this.set_initial_focus(b);
        this.set_cancel_code(MENU_CODE_MAP_BROWSER_EXIT);
        this
    }
}
