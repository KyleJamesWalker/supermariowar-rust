//! Options > Stats: the lifetime totals of this device's human players (see `smw::stats`), and Reset Stats.

use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{ControlId, Ctl, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::game::Game;
use crate::smw::stats::{clock, Lifetime};

/// Label and value text rows laid out on screen.
const ROWS: usize = 16;

/// Lifetime stats table with Reset Stats and a confirmation dialog.
#[derive(Default)]
pub struct UI_StatsMenu {
    pub ui_menu: UI_Menu,
    labels: Vec<Ctl<MI_Text>>,
    values: Vec<Ctl<MI_Text>>,
    reset: Ctl<MI_Button>,
    back: Ctl<MI_Button>,
    left_bar: Ctl<MI_Image>,
    right_bar: Ctl<MI_Image>,
    header: Ctl<MI_Text>,
    dialog: Ctl<MI_Image>,
    dialog_are_you: Ctl<MI_Text>,
    dialog_sure: Ctl<MI_Text>,
    yes: Ctl<MI_Button>,
    no: Ctl<MI_Button>,
}
crate::impl_base!(UI_StatsMenu => ui_menu: UI_Menu);

/// The rows the screen shows: a label and its value.
fn rows(life: &Lifetime) -> Vec<(String, String)> {
    let mut rows = vec![
        ("Matches played".to_string(), life.total("matches").to_string()),
        ("Matches won".to_string(), life.total("wins").to_string()),
        ("Kills".to_string(), life.total("kills").to_string()),
        ("Deaths".to_string(), life.total("deaths").to_string()),
        ("Suicides".to_string(), life.total("suicides").to_string()),
        ("Air doubles".to_string(), life.total("air_doubles").to_string()),
        ("Air triples".to_string(), life.total("air_triples").to_string()),
        ("Jumps".to_string(), life.total("jumps").to_string()),
        ("Air time".to_string(), clock(life.total("air_ticks"))),
        ("Star time".to_string(), clock(life.total("star_ticks"))),
        ("Powerups, items used".to_string(), format!("{}, {}", life.total("powerups"), life.total("items_used"))),
        ("Most kills in a match".to_string(), life.best_of("kills_in_match").to_string()),
        ("Longest kill streak".to_string(), life.best_of("kill_streak").to_string()),
        ("Longest life".to_string(), clock(life.best_of("survival_ticks"))),
        ("Most air doubles in a match".to_string(), life.best_of("air_doubles_in_match").to_string()),
        ("Most air triples in a match".to_string(), life.best_of("air_triples_in_match").to_string()),
    ];
    let favourite = |table: &std::collections::BTreeMap<String, (u64, u64)>| {
        table.iter().max_by_key(|(name, (played, _))| (*played, std::cmp::Reverse((*name).clone()))).map(|(name, (p, w))| format!("{name} ({w}/{p})"))
    };
    rows.push(("Most played mode (won)".to_string(), favourite(&life.modes).unwrap_or_else(|| "-".to_string())));
    rows.push(("Most played map (won)".to_string(), favourite(&life.maps).unwrap_or_else(|| "-".to_string())));
    rows
}

impl UI_StatsMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        let spr_selectfield = g.rm.spr_selectfield;
        let menu_plain_field = g.rm.menu_plain_field;

        for i in 0..ROWS {
            let y = 44 + i as i16 * 22;
            let label = Ctl::new(MI_Text::new(g, "", 40, y, 0, false, TextAlign::LEFT), g);
            let value = Ctl::new(MI_Text::new(g, "", 600, y, 0, false, TextAlign::RIGHT), g);
            this.add_non_control(label.id());
            this.add_non_control(value.id());
            this.labels.push(label);
            this.values.push(value);
        }

        this.reset = Ctl::new(MI_Button::new(g, spr_selectfield, 16, 432, "Reset Stats", 200, TextAlign::CENTER), g);
        g.ctl(this.reset).set_code(MENU_CODE_STATS_RESET);
        this.back = Ctl::new(MI_Button::new(g, spr_selectfield, 544, 432, "Back", 80, TextAlign::CENTER), g);
        g.ctl(this.back).set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

        this.left_bar = Ctl::new(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
        this.right_bar = Ctl::new(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
        this.header = Ctl::new(MI_HeaderText::new(g, "Stats", 320, 5), g);

        this.dialog = Ctl::new(MI_Image::new(g.rm.spr_dialog, 224, 176, 0, 0, 192, 128, 1, 1, 0), g);
        this.dialog_are_you = Ctl::new(MI_HeaderText::new(g, "Reset All", 320, 195), g);
        this.dialog_sure = Ctl::new(MI_HeaderText::new(g, "Stats?", 320, 220), g);
        this.yes = Ctl::new(MI_Button::new(g, spr_selectfield, 235, 250, "Yes", 80, TextAlign::CENTER), g);
        this.no = Ctl::new(MI_Button::new(g, spr_selectfield, 325, 250, "No", 80, TextAlign::CENTER), g);
        g.ctl(this.yes).set_code(MENU_CODE_STATS_RESET_YES);
        g.ctl(this.no).set_code(MENU_CODE_STATS_RESET_NO);

        let (reset, back, yes, no) = (this.reset.id(), this.back.id(), this.yes.id(), this.no.id());
        let null = ControlId::NONE;
        this.add_control(g, reset, null, null, null, back);
        this.add_control(g, back, null, null, reset, null);
        for c in [this.left_bar.id(), this.right_bar.id(), this.header.id(), this.dialog.id(), this.dialog_are_you.id(), this.dialog_sure.id()] {
            this.add_non_control(c);
        }
        this.add_control(g, yes, null, null, null, no);
        this.add_control(g, no, null, null, yes, null);
        this.show_dialog(g, false);

        this.set_initial_focus(g, back);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }

    /// Shows or hides the Reset All Stats dialog.
    fn show_dialog(&mut self, g: &mut Game, on: bool) {
        g.ctl(self.dialog).set_visible(on);
        g.ctl(self.dialog_are_you).set_visible(on);
        g.ctl(self.dialog_sure).set_visible(on);
        g.ctl(self.yes).set_visible(on);
        g.ctl(self.no).set_visible(on);
    }

    /// Reads the saved totals into the rows.
    pub fn refresh(&mut self, g: &mut Game) {
        let life = crate::smw::stats::load(g);
        let rows = rows(&life);
        for i in 0..ROWS {
            let (label, value) = rows.get(i).cloned().unwrap_or_default();
            g.ctl(self.labels[i]).set_text(label);
            g.ctl(self.values[i]).set_text(value);
        }
    }

    /// Opens the reset confirmation with No focused, remembering the focus to restore.
    pub fn open_reset_dialog(&mut self, g: &mut Game) {
        self.show_dialog(g, true);
        self.remember_current();
        let focus = self.no.id();
        self.set_initial_focus(g, focus);
        self.set_cancel_code(MENU_CODE_STATS_RESET_NO);
    }

    /// Closes the confirmation; `reset` wipes the saved lifetime stats and refreshes the rows.
    pub fn close_reset_dialog(&mut self, g: &mut Game, reset: bool) {
        if reset {
            crate::smw::stats::reset_lifetime(g);
            self.refresh(g);
        }
        self.show_dialog(g, false);
        let focus = self.back.id();
        self.set_initial_focus(g, focus);
        self.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        self.restore_current(g);
    }
}
