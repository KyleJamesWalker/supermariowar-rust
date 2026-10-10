//! Port of src/common/uimenu.cpp
//!
//! `UI_Menu` has no virtuals besides its destructor, so menus embed it as their base (`impl_base!`) and
//! `GSMenu` refers to the current one as `Ptr<UI_Menu>`. Controls are `new`ed by the menus, which keep
//! raw pointers to them; the menu owns and deletes them, like the C++ `unique_ptr` vector.

use crate::common::eyecandy::CEyecandyContainer;
use crate::common::input::{CPlayerInput, DEVICE_KEYBOARD};
use crate::common::ui::menu_code::*;
use crate::common::uicontrol::{ControlPtr, UI_ControlTrait};
use crate::globals::*;
use crate::smw::gs_gameplay::lookup_team_id;

const fn nav_dir2menu_code(direction: MenuNavDirection) -> MenuCodeEnum {
    match direction {
        MenuNavDirection::Up => MENU_CODE_NEIGHBOR_UP,
        MenuNavDirection::Down => MENU_CODE_NEIGHBOR_DOWN,
        MenuNavDirection::Left => MENU_CODE_NEIGHBOR_LEFT,
        MenuNavDirection::Right => MENU_CODE_NEIGHBOR_RIGHT,
    }
}

pub struct UI_Menu {
    pub controls: Vec<ControlPtr>,

    pub m_initialFocus: ControlPtr,
    pub m_currentFocus: ControlPtr,
    pub m_savedCurrent: ControlPtr,

    pub cancelCode: MenuCodeEnum,
    pub fModifyingItem: bool,

    pub eyeCandy: CEyecandyContainer,

    pub iControllingTeam: i16,
    pub fAllowExitButton: bool,

    pub _alias: Aliased,
}

impl Drop for UI_Menu {
    fn drop(&mut self) {
        for control in self.controls.drain(..) {
            control.delete();
        }
    }
}

impl Default for UI_Menu {
    fn default() -> Self {
        Self::new()
    }
}

impl UI_Menu {
    pub fn new() -> Self {
        UI_Menu {
            controls: Vec::new(),
            m_initialFocus: Ptr::null(),
            m_currentFocus: Ptr::null(),
            m_savedCurrent: Ptr::null(),
            cancelCode: MENU_CODE_NONE,
            fModifyingItem: false,
            eyeCandy: CEyecandyContainer::new(),
            iControllingTeam: -1,
            fAllowExitButton: true,
            _alias: Aliased::new(),
        }
    }

    /// Takes ownership of `control` (a `Ptr::new_box`ed control), like `controls.emplace_back(control)`.
    pub fn add_control(&mut self, mut control: ControlPtr, up: ControlPtr, down: ControlPtr, left: ControlPtr, right: ControlPtr) {
        let this = Ptr::from_mut(self);
        let c = control.ctl_mut();
        c.set_parent(this);
        c.set_neighbor(MenuNavDirection::Up, up);
        c.set_neighbor(MenuNavDirection::Down, down);
        c.set_neighbor(MenuNavDirection::Left, left);
        c.set_neighbor(MenuNavDirection::Right, right);
        self.controls.push(control);
    }

    pub fn add_non_control(&mut self, mut control: ControlPtr) {
        let this = Ptr::from_mut(self);
        control.ctl_mut().set_parent(this);
        self.controls.push(control);
    }

    /// Sets the initially focused element on opening/resetting the menu.
    pub fn set_initial_focus(&mut self, control: ControlPtr) {
        self.m_initialFocus = control;
        self.reset_menu();
    }
    /// The initially focused element on opening/resetting the menu.
    pub fn initial_focus(&self) -> ControlPtr {
        self.m_initialFocus
    }
    /// The currently focused element of the menu.
    pub fn current_focus(&self) -> ControlPtr {
        self.m_currentFocus
    }

    pub fn set_cancel_code(&mut self, code: MenuCodeEnum) {
        self.cancelCode = code;
    }

    pub fn reset_menu(&mut self) {
        if !self.m_currentFocus.is_null() {
            self.m_currentFocus.modify(false);
            self.m_currentFocus.select(false);
        }

        self.m_currentFocus = self.m_initialFocus;

        if !self.m_currentFocus.is_null() {
            self.fModifyingItem = self.m_currentFocus.select(true);
        }

        self.eyeCandy.clean();
    }

    pub fn update(&mut self) {
        for i in 0..self.controls.len() {
            self.controls[i].update();
        }

        self.eyeCandy.clean_dead_objects();
        self.eyeCandy.update();
    }

    pub fn draw(&mut self) {
        for i in 0..self.controls.len() {
            self.controls[i].draw();
        }

        self.eyeCandy.draw();

        if self.fModifyingItem && !self.m_currentFocus.is_null() {
            self.m_currentFocus.draw_overlay();
        }
    }

    pub fn send_input(&mut self, playerInput: Ptr<CPlayerInput>) -> MenuCodeEnum {
        unsafe {
            if self.fModifyingItem {
                let ret = self.m_currentFocus.send_input(playerInput);

                if MENU_CODE_UNSELECT_ITEM == ret {
                    self.fModifyingItem = false;

                    /*if (current->IsAutoModify())
                            return cancelCode;*/

                    return MENU_CODE_NONE;
                }

                return ret;
            }

            for iPlayer in 0..4i16 {
                let out = &playerInput.outputControls[iPlayer as usize];
                // Only allow the controlling team to control the menu (if there is one)
                if self.iControllingTeam != -1 {
                    // Pay attention to other player's exit button pushes so we can exit when AI is controlling
                    if game_values.playercontrol[iPlayer as usize] != 1
                        || (self.iControllingTeam != lookup_team_id(iPlayer) && (!self.fAllowExitButton || !out.menu_cancel().fPressed))
                    {
                        continue;
                    }
                }
                // Only let player 1 on the keyboard control the menu unless there is another controlling team
                // Not in upstream: the first keyboard player may too, when joysticks took the players before it.
                else if iPlayer != 0
                    && !game_values.playerInput.inputControls[iPlayer as usize].is_null()
                    && game_values.playerInput.inputControls[iPlayer as usize].iDevice == DEVICE_KEYBOARD
                    && (0..iPlayer as usize).any(|p| game_values.playerInput.inputControls[p].is_null() || game_values.playerInput.inputControls[p].iDevice == DEVICE_KEYBOARD)
                {
                    continue;
                }

                if out.menu_up().fPressed {
                    return self.move_next_control(MenuNavDirection::Up);
                }

                if out.menu_down().fPressed {
                    return self.move_next_control(MenuNavDirection::Down);
                }

                if out.menu_left().fPressed {
                    return self.move_next_control(MenuNavDirection::Left);
                }

                if out.menu_right().fPressed {
                    return self.move_next_control(MenuNavDirection::Right);
                }

                if out.menu_select().fPressed {
                    let mut ret = MENU_CODE_NONE;

                    if !self.m_currentFocus.is_null() {
                        crate::common::ui::mi_text_field::note_opener(playerInput.iPressedKey != 0);
                        ret = self.m_currentFocus.modify(true);

                        if MENU_CODE_MODIFY_ACCEPTED == ret {
                            self.fModifyingItem = true;
                            return MENU_CODE_NONE;
                        }

                        if MENU_CODE_UNSELECT_ITEM == ret {
                            self.fModifyingItem = false;
                            return MENU_CODE_NONE;
                        }
                    }

                    return ret;
                }

                if out.menu_cancel().fPressed {
                    return self.cancelCode;
                }
            }

            MENU_CODE_NONE
        }
    }

    pub fn move_next_control(&mut self, iDirection: MenuNavDirection) -> MenuCodeEnum {
        if self.m_currentFocus.is_null() {
            return MENU_CODE_NONE;
        }

        let mut neighbor = self.m_currentFocus.ctl().neighbor(iDirection);

        while !neighbor.is_null() && !neighbor.ctl().is_visible() {
            neighbor = neighbor.ctl().neighbor(iDirection);
        }

        if !neighbor.is_null() {
            self.m_currentFocus.select(false);
            self.m_currentFocus = neighbor;
            self.fModifyingItem = self.m_currentFocus.select(true);
            return nav_dir2menu_code(iDirection);
        }

        MENU_CODE_NONE
    }

    pub fn remember_current(&mut self) {
        self.m_savedCurrent = self.m_currentFocus;
    }

    pub fn restore_current(&mut self) {
        if !self.m_currentFocus.is_null() {
            self.m_currentFocus.modify(false);
            self.m_currentFocus.select(false);
        }

        self.m_currentFocus = self.m_savedCurrent;

        if !self.m_currentFocus.is_null() {
            self.fModifyingItem = self.m_currentFocus.select(true);
        }

        self.eyeCandy.clean();
    }

    pub fn set_controlling_team(&mut self, teamid: i16) {
        self.iControllingTeam = teamid;
    }
    pub fn set_allow_exit(&mut self, allowExit: bool) {
        self.fAllowExitButton = allowExit;
    }

    pub fn mouse_click(&mut self, iMouseX: i16, iMouseY: i16) -> MenuCodeEnum {
        // Loop through all controls to see if one was clicked on
        let mut pFound: ControlPtr = Ptr::null();
        let mut code = MENU_CODE_NONE;
        for i in 0..self.controls.len() {
            let mut control = self.controls[i];
            if control.ctl().is_visible() {
                code = control.mouse_click(iMouseX, iMouseY);
                if code != MENU_CODE_NONE {
                    pFound = control;
                    break;
                }
            }
        }

        if !pFound.is_null() {
            crate::common::ui::mi_text_field::note_opener(false);
            // If we clicked the same control we have selected
            if pFound != self.m_currentFocus {
                if self.fModifyingItem {
                    self.m_currentFocus.modify(false);
                    self.fModifyingItem = false;
                }

                self.m_currentFocus.select(false);
                self.m_currentFocus = pFound;
                self.fModifyingItem = self.m_currentFocus.select(true);

                if !self.fModifyingItem {
                    self.fModifyingItem = self.m_currentFocus.modify(true) == MENU_CODE_MODIFY_ACCEPTED;
                }
            } else if !self.fModifyingItem {
                self.fModifyingItem = self.m_currentFocus.modify(true) == MENU_CODE_MODIFY_ACCEPTED;
            }
        } else {
            // If nothing was clicked, then stop modifying the current control
            if self.fModifyingItem {
                self.m_currentFocus.modify(false);
                self.fModifyingItem = false;
            }
        }

        code
    }

    pub fn is_modifying(&self) -> bool {
        self.fModifyingItem
    }

    pub fn refresh(&mut self) {
        for i in 0..self.controls.len() {
            self.controls[i].refresh();
        }
    }

    /// Replay harness (cpp-harness.patch): index of the focused element in insertion order, or -1.
    pub fn current_focus_index(&self) -> i32 {
        for (i, c) in self.controls.iter().enumerate() {
            if *c == self.m_currentFocus {
                return i as i32;
            }
        }
        -1
    }
}
