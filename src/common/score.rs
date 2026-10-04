//! Port of src/common/Score.h

use crate::globals::Aliased;
/// `CScore::AdjustScore` is defined in smw/player.cpp, so its port lives in `smw::player`.
#[derive(Clone, Debug, Default)]
pub struct CScore {
    pub score: i16,
    pub subscore: [i16; 3],
    pub x: i16,
    pub y: i16,
    pub destx: i16,
    pub desty: i16,
    pub place: i16,
    pub displayorder: i16,
    pub order: i16,
    pub fromx: i16,
    pub fromy: i16,
    pub iDigitRight: i16,
    pub iDigitMiddle: i16,
    pub iDigitLeft: i16,
    pub _alias: Aliased,
}

impl CScore {
    pub fn new(iPlace: i16) -> Self {
        CScore { place: iPlace, displayorder: iPlace, ..Default::default() }
    }

    pub fn set_score(&mut self, iValue: i16) {
        if crate::smw::net_outcomes::score_locked() {
            return;
        }
        self.score = iValue;
        self.set_digit_counters();
    }

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
