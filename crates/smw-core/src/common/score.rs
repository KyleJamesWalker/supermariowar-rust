//! A team's score and its scoreboard display state.

use crate::game::Game;
use crate::globals::Aliased;
/// A team's score and where its scoreboard entry is drawn; `adjust_score` lives in `smw::player`.
#[derive(Clone, Debug, Default)]
pub struct CScore {
    /// The mode's score; tied scores rank by `order`.
    pub score: i16,
    /// Extra per-mode counters; the boxes minigame ranks by `subscore[0]`.
    pub subscore: [i16; 3],
    /// The scoreboard entry's position in pixels.
    pub x: i16,
    pub y: i16,
    /// Where the entry moves to when the scoreboard is shown at match end.
    pub destx: i16,
    pub desty: i16,
    /// Finishing place, 0 for first.
    pub place: i16,
    /// Row on the end-of-match scoreboard.
    pub displayorder: i16,
    /// When the team was removed (`teamdeadcounter`), or -1 while it is alive; later is better on a tie.
    pub order: i16,
    /// Where the entry starts moving from towards `destx`, `desty`.
    pub fromx: i16,
    pub fromy: i16,
    /// The x offsets in the score digits sprite for the three digits shown; 16 pixels per digit.
    pub iDigitRight: i16,
    pub iDigitMiddle: i16,
    pub iDigitLeft: i16,
    pub _alias: Aliased,
}

impl CScore {
    pub fn new(iPlace: i16) -> Self {
        CScore { place: iPlace, displayorder: iPlace, ..Default::default() }
    }

    pub fn set_score(&mut self, g: &mut Game, iValue: i16) {
        self.score = iValue;
        self.set_digit_counters();
    }

    /// Recomputes the digit offsets for `score`, showing its last three digits.
    pub fn set_digit_counters(&mut self) {
        let mut iDigits: i16 = self.score;
        while iDigits > 999 {
            iDigits -= 1000;
        }

        self.iDigitLeft = iDigits / 100 * 16;
        self.iDigitMiddle = iDigits % 100 / 10 * 16;
        self.iDigitRight = iDigits % 10 * 16;
    }
}
