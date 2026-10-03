//! Port of src/common/uicontrol.cpp
//!
//! `UI_Control` is the data of the C++ base class; `UI_ControlTrait` holds its virtuals. Every menu item
//! embeds its base as the first field (`impl_base!`) and implements the trait, returning the embedded
//! `UI_Control` from `ctl()`/`ctl_mut()`. Controls are referenced as `Ptr<dyn UI_ControlTrait>`.

use crate::common::input::CPlayerInput;
use crate::common::math::vec2::Vec2s;
use crate::common::ui::menu_code::*;
use crate::common::uimenu::UI_Menu;
use crate::globals::*;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum TextAlign {
    #[default]
    LEFT,
    CENTER,
    RIGHT,
}

pub type ControlPtr = Ptr<dyn UI_ControlTrait>;

pub struct UI_Control {
    pub m_pos: Vec2s,

    pub fSelected: bool,
    pub fModifying: bool,
    pub fAutoModify: bool,
    pub fDisable: bool,
    pub m_visible: bool,

    pub m_neighbors: [ControlPtr; 4],

    pub m_parentMenu: Ptr<UI_Menu>,
    pub iControllingTeam: i16,

    pub _alias: Aliased,
}

impl UI_Control {
    pub fn new(x: i16, y: i16) -> Self {
        UI_Control {
            m_pos: Vec2s::new(x, y),
            fSelected: false,
            fModifying: false,
            fAutoModify: false,
            fDisable: false,
            m_visible: true,
            m_neighbors: [Ptr::null(); 4],
            m_parentMenu: Ptr::null(),
            iControllingTeam: -1,
            _alias: Aliased::new(),
        }
    }

    /// `UI_Control::operator=` / copy constructor: parent and neighbors are not copied.
    pub fn copy_from(other: &UI_Control) -> Self {
        UI_Control {
            m_pos: other.m_pos,
            fSelected: other.fSelected,
            fModifying: other.fModifying,
            fAutoModify: other.fAutoModify,
            fDisable: other.fDisable,
            m_visible: other.m_visible,
            m_neighbors: [Ptr::null(); 4],
            m_parentMenu: Ptr::null(),
            iControllingTeam: other.iControllingTeam,
            _alias: Aliased::new(),
        }
    }

    pub fn set_auto_modify(&mut self, autoModify: bool) {
        self.fAutoModify = autoModify;
    }
    pub fn is_auto_modify(&self) -> bool {
        self.fAutoModify
    }

    pub fn set_position(&mut self, x: i16, y: i16) {
        self.m_pos = Vec2s::new(x, y);
    }

    pub fn set_neighbor(&mut self, iNeighbor: MenuNavDirection, uiControl: ControlPtr) {
        self.m_neighbors[iNeighbor as usize] = uiControl;
    }

    pub fn neighbor(&self, iNeighbor: MenuNavDirection) -> ControlPtr {
        self.m_neighbors[iNeighbor as usize]
    }

    pub fn set_visible(&mut self, show: bool) {
        self.m_visible = show;
    }
    pub fn is_visible(&self) -> bool {
        self.m_visible
    }

    pub fn set_parent(&mut self, menu: Ptr<UI_Menu>) {
        self.m_parentMenu = menu;
    }

    pub fn is_modifying(&self) -> bool {
        self.fModifying
    }

    pub fn set_controlling_team(&mut self, teamid: i16) {
        self.iControllingTeam = teamid;
    }
}

/// `UI_Control::Modify` (the base-class body), for overrides that call `UI_Control::Modify(modify)`.
pub fn ui_control_modify<T: UI_ControlTrait + ?Sized>(this: &mut T, modify: bool) -> MenuCodeEnum {
    let c = this.ctl_mut();
    if c.fDisable {
        return MENU_CODE_UNSELECT_ITEM;
    }

    c.fModifying = modify;
    MENU_CODE_MODIFY_ACCEPTED
}

pub trait UI_ControlTrait {
    fn ctl(&self) -> &UI_Control;
    fn ctl_mut(&mut self) -> &mut UI_Control;

    /// Updates animations or other events every frame
    fn update(&mut self) {}

    /// Draws every frame
    fn draw(&mut self) {}

    /// Sends player input to control on every frame
    fn send_input(&mut self, _playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        MENU_CODE_NONE
    }

    /// Called when user selects this control to change it's value
    fn modify(&mut self, modify: bool) -> MenuCodeEnum {
        ui_control_modify(self, modify)
    }

    fn mouse_click(&mut self, _iMouseX: i16, _iMouseY: i16) -> MenuCodeEnum {
        MENU_CODE_NONE
    }

    fn refresh(&mut self) {}

    fn disable(&mut self, disable: bool) {
        self.ctl_mut().fDisable = disable;
    }

    /// Non-virtual in C++.
    fn select(&mut self, select: bool) -> bool {
        self.ctl_mut().fSelected = select;

        if self.ctl().fSelected && self.ctl().fAutoModify {
            self.modify(true);
        }

        self.ctl().fModifying
    }
}

/// Implements `UI_ControlTrait::ctl`/`ctl_mut` for a control whose base chain ends in `UI_Control`.
#[macro_export]
macro_rules! impl_ctl {
    () => {
        fn ctl(&self) -> &$crate::common::uicontrol::UI_Control {
            self
        }
        fn ctl_mut(&mut self) -> &mut $crate::common::uicontrol::UI_Control {
            self
        }
    };
}

impl UI_ControlTrait for UI_Control {
    fn ctl(&self) -> &UI_Control {
        self
    }
    fn ctl_mut(&mut self) -> &mut UI_Control {
        self
    }
}

/// `UI_Control*` from a concrete control pointer.
pub fn ctl_ptr<T: UI_ControlTrait + 'static>(p: Ptr<T>) -> ControlPtr {
    if p.is_null() {
        Ptr::null()
    } else {
        Ptr::from_raw(p.as_ptr() as *mut dyn UI_ControlTrait)
    }
}
