//! Menu controls: the shared control data, their virtual interface, and the store that owns them.
//!
//! `UI_Control` is the data every menu item shares; `UI_ControlTrait` holds its overridable behavior. Every menu item
//! embeds its base as the first field (`impl_base!`) and implements the trait, returning the embedded
//! `UI_Control` from `ctl()`/`ctl_mut()`. `Game::controls` owns every control; menus hold `ControlId`s.

use crate::game::Game;
use crate::common::input::CPlayerInput;
use crate::common::math::vec2::Vec2s;
use crate::common::ui::menu_code::*;
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use crate::globals::*;

/// Horizontal text alignment within a control.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum TextAlign {
    #[default]
    LEFT,
    CENTER,
    RIGHT,
}

/// A control in `Game::controls`. A handle to a removed control finds nothing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ControlId {
    slot: u16,
    generation: u16,
}

impl ControlId {
    /// No control.
    pub const NONE: ControlId = ControlId { slot: u16::MAX, generation: 0 };

    pub fn is_none(self) -> bool {
        self == Self::NONE
    }
}

impl Default for ControlId {
    fn default() -> Self {
        Self::NONE
    }
}

/// A `ControlId` that `Controls::insert` made for a `T`.
pub struct Ctl<T: ?Sized> {
    id: ControlId,
    _type: PhantomData<fn() -> *const T>,
}

impl<T: ?Sized> Ctl<T> {
    pub const NONE: Self = Ctl { id: ControlId::NONE, _type: PhantomData };

    pub fn id(self) -> ControlId {
        self.id
    }

    pub fn is_none(self) -> bool {
        self.id.is_none()
    }
}

impl<T: UI_ControlTrait + 'static> Ctl<T> {
    pub fn new(control: T, g: &mut Game) -> Self {
        g.controls.insert(control)
    }
}

impl<T: ?Sized> Clone for Ctl<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for Ctl<T> {}

impl<T: ?Sized> PartialEq for Ctl<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: ?Sized> Default for Ctl<T> {
    fn default() -> Self {
        Self::NONE
    }
}

impl<T: ?Sized> std::fmt::Debug for Ctl<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.id.fmt(f)
    }
}

/// Every menu's controls. Menus live for the whole run, so a dropped menu leaves its controls here.
#[derive(Default)]
pub struct Controls {
    slots: Vec<Option<Box<UnsafeCell<dyn UI_ControlTrait>>>>,
    generations: Vec<u16>,
}

impl Controls {
    pub const fn new() -> Self {
        Controls { slots: Vec::new(), generations: Vec::new() }
    }

    /// Stores `control` in a free slot and returns a typed handle to it.
    pub fn insert<T: UI_ControlTrait + 'static>(&mut self, control: T) -> Ctl<T> {
        let slot = match self.slots.iter().position(Option::is_none) {
            Some(slot) => slot,
            None => {
                self.slots.push(None);
                self.generations.push(0);
                self.slots.len() - 1
            }
        };
        self.slots[slot] = Some(Box::new(UnsafeCell::new(control)));
        Ctl { id: ControlId { slot: slot as u16, generation: self.generations[slot] }, _type: PhantomData }
    }

    /// Frees the control; handles to it find nothing from now on.
    pub fn remove(&mut self, id: ControlId) {
        if self.get(id).is_some() {
            self.slots[id.slot as usize] = None;
            self.generations[id.slot as usize] = self.generations[id.slot as usize].wrapping_add(1);
        }
    }

    #[allow(clippy::mut_from_ref)]
    /// The control `id` names, or `None` if it was removed.
    pub fn get(&self, id: ControlId) -> Option<&'static mut dyn UI_ControlTrait> {
        let slot = id.slot as usize;
        if slot >= self.slots.len() || self.generations[slot] != id.generation {
            return None;
        }
        let control = self.slots[slot].as_deref()?.get();
        Some(unsafe { &mut *control })
    }

    /// The control `ctl` names, as its type; panics on a removed control.
    #[allow(clippy::mut_from_ref)]
    pub fn typed<T: 'static>(&self, ctl: Ctl<T>) -> &'static mut T {
        let control = self.get(ctl.id).unwrap_or_else(|| panic!("no control {:?}", ctl.id));
        unsafe { &mut *(control as *mut dyn UI_ControlTrait as *mut T) }
    }
}

/// What every menu control has: position, selection state and neighbors for menu navigation.
pub struct UI_Control {
    /// Position in pixels on the 640x480 screen.
    pub m_pos: Vec2s,

    /// Highlighted as the menu's current control.
    pub fSelected: bool,
    /// Taking input itself, such as an open select field.
    pub fModifying: bool,
    /// Starts modifying as soon as it is selected.
    pub fAutoModify: bool,
    pub fDisable: bool,
    pub m_visible: bool,

    /// The controls the menu moves to, by `MenuNavDirection`.
    pub m_neighbors: [ControlId; 4],

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
            m_neighbors: [ControlId::NONE; 4],
            iControllingTeam: -1,
            _alias: Aliased::new(),
        }
    }

    /// A copy that keeps the state but no neighbors.
    pub fn copy_from(other: &UI_Control) -> Self {
        UI_Control {
            m_pos: other.m_pos,
            fSelected: other.fSelected,
            fModifying: other.fModifying,
            fAutoModify: other.fAutoModify,
            fDisable: other.fDisable,
            m_visible: other.m_visible,
            m_neighbors: [ControlId::NONE; 4],
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

    pub fn set_neighbor(&mut self, iNeighbor: MenuNavDirection, uiControl: ControlId) {
        self.m_neighbors[iNeighbor as usize] = uiControl;
    }

    pub fn neighbor(&self, iNeighbor: MenuNavDirection) -> ControlId {
        self.m_neighbors[iNeighbor as usize]
    }

    pub fn set_visible(&mut self, show: bool) {
        self.m_visible = show;
    }
    pub fn is_visible(&self) -> bool {
        self.m_visible
    }

    pub fn is_modifying(&self) -> bool {
        self.fModifying
    }

    pub fn set_controlling_team(&mut self, teamid: i16) {
        self.iControllingTeam = teamid;
    }
}

/// The default `modify`: refuses while disabled, else sets `fModifying`; for overrides that also want it.
pub fn ui_control_modify<T: UI_ControlTrait + ?Sized>(this: &mut T, modify: bool) -> MenuCodeEnum {
    let c = this.ctl_mut();
    if c.fDisable {
        return MENU_CODE_UNSELECT_ITEM;
    }

    c.fModifying = modify;
    MENU_CODE_MODIFY_ACCEPTED
}

/// The overridable behavior of a menu control.
pub trait UI_ControlTrait {
    fn ctl(&self) -> &UI_Control;
    fn ctl_mut(&mut self) -> &mut UI_Control;

    /// Updates animations or other events every frame.
    fn update(&mut self, g: &mut Game) {}

    /// Draws every frame.
    fn draw(&mut self, g: &mut Game) {}

    /// Drawn after the whole menu while this control is being modified.
    fn draw_overlay(&mut self, g: &mut Game) {}

    /// Takes this frame's player input while the control is being modified.
    fn send_input(&mut self, g: &mut Game, _playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        MENU_CODE_NONE
    }

    /// Called when the user selects this control to change its value, or ends modifying it.
    fn modify(&mut self, g: &mut Game, modify: bool) -> MenuCodeEnum {
        ui_control_modify(self, modify)
    }

    /// A mouse click or tap at `_iMouseX`, `_iMouseY` in screen pixels.
    fn mouse_click(&mut self, g: &mut Game, _iMouseX: i16, _iMouseY: i16) -> MenuCodeEnum {
        MENU_CODE_NONE
    }

    /// Reloads the shown value from what it controls.
    fn refresh(&mut self, g: &mut Game) {}

    fn disable(&mut self, disable: bool) {
        self.ctl_mut().fDisable = disable;
    }

    /// Sets the selection; starts modifying if the control auto-modifies. Returns whether it is modifying.
    fn select(&mut self, g: &mut Game, select: bool) -> bool {
        self.ctl_mut().fSelected = select;

        if self.ctl().fSelected && self.ctl().fAutoModify {
            self.modify(g, true);
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
