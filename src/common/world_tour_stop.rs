//! Port of src/common/WorldTourStop.cpp

use crate::common::game_mode::*;
use crate::common::game_mode_settings::*;
use crate::common::game_mode_settings_serialization::{deserialize_gms, serialize_gms};
use crate::common::gameplay_styles::*;
use crate::common::global::{game_values, maplist};
use crate::common::global_constants::*;
use crate::common::match_types::Boss;
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::version::Version;
use crate::globals::{Ptr, Aliased};
use crate::smw::main::gamemodes;

const DELIM_EMPTY: &str = "";
const DELIM_COMMA: &str = ",";
const DELIM_PIPE: &str = "|";
const UNKNOWN_MAP_NAME: &str = "-";

#[derive(Clone, Debug, Default)]
pub struct WorldStageBonus {
    pub iWinnerPlace: i16,
    pub iBonus: i16,
    pub szBonusString: String,
}

#[derive(Clone, Debug, Default)]
pub struct TourStop {
    pub pszMapFile: String,
    pub iMode: i16,
    pub iGoal: i16,
    pub iPoints: i16,
    pub iBonusType: i16,
    pub szName: String,

    pub fEndStage: bool,
    pub iNumBonuses: i16,
    pub wsbBonuses: [WorldStageBonus; 10],
    pub iStageType: i16,

    pub fUseSettings: bool,
    pub iNumUsedSettings: i16,
    pub gmsSettings: GameModeSettings,

    pub iBonusTextLines: i16,
    pub szBonusText: [String; 5],
    pub _alias: Aliased,
}

/// C `strtok` over a NUL-terminated byte buffer; positions are indices into `buf`.
pub struct Strtok {
    pub buf: Vec<u8>,
    next: Option<usize>,
}

impl Strtok {
    /// Copies `s` and appends the terminating NUL.
    pub fn new(s: &[u8]) -> Self {
        let mut buf = s.to_vec();
        buf.push(0);
        Strtok { buf, next: Some(0) }
    }

    /// `strtok(first ? buffer : NULL, delims)`
    pub fn tok(&mut self, delims: &[u8]) -> Option<usize> {
        let mut p = self.next?;
        while self.buf[p] != 0 && delims.contains(&self.buf[p]) {
            p += 1;
        }
        if self.buf[p] == 0 {
            self.next = None;
            return None;
        }
        let start = p;
        while self.buf[p] != 0 && !delims.contains(&self.buf[p]) {
            p += 1;
        }
        if self.buf[p] != 0 {
            self.buf[p] = 0;
            self.next = Some(p + 1);
        } else {
            self.next = None;
        }
        Some(start)
    }

    /// The C string starting at `p`.
    pub fn cstr(&self, p: usize) -> String {
        let end = p + self.buf[p..].iter().position(|&c| c == 0).unwrap_or(self.buf.len() - p);
        crate::common::file_io::cstr_bytes_to_string(&self.buf[p..end])
    }

    pub fn at(&self, p: usize) -> u8 {
        self.buf.get(p).copied().unwrap_or(0)
    }

    pub fn atoi(&self, p: usize) -> i32 {
        crate::common::map_list::atoi(&self.cstr(p))
    }

    /// `strstr(p, needle)` for a single-byte needle.
    pub fn strchr(&self, p: usize, c: u8) -> Option<usize> {
        let mut i = p;
        while self.buf[i] != 0 {
            if self.buf[i] == c {
                return Some(i);
            }
            i += 1;
        }
        None
    }
}

/// `ParseTourStopLine(char* buffer, ...)`; `buffer` is the line as read by `fgets`, without the NUL.
pub fn parse_tour_stop_line(buffer: &[u8], version: &Version, fIsWorld: bool) -> TourStop {
    let mut st = Strtok::new(buffer);
    let mut ts = TourStop { fUseSettings: false, iNumUsedSettings: 0, ..Default::default() };

    let mut pszTemp = st.tok(b",\n");

    ts.iStageType = 0;
    if fIsWorld {
        ts.iStageType = st.atoi(pszTemp.expect("strtok returned NULL")) as i16;
        if ts.iStageType < 0 || ts.iStageType > 1 {
            ts.iStageType = 0;
        }

        pszTemp = st.tok(b",\n");
    }

    for t in ts.szBonusText.iter_mut() {
        t.clear();
    }

    let v = |major, minor, patch, build| Version { major, minor, patch, build };

    if ts.iStageType == 0 {
        let szMap = st.cstr(pszTemp.expect("strtok returned NULL"));
        pszTemp = st.tok(b",\n");

        ts.iMode = match pszTemp {
            Some(p) => st.atoi(p) as i16,
            None => -1,
        };

        let mode = ts.iMode as i32;
        let isMinigame = mode == game_mode_pipe_minigame || mode == game_mode_boss_minigame || mode == game_mode_boxes_minigame;

        unsafe {
            if *version <= v(1, 8, 0, 2) && isMinigame {
                ts.pszMapFile = UNKNOWN_MAP_NAME.to_string();
            } else {
                maplist.save_current();

                let fMapFound = maplist.findexact(&szMap, true);
                if !fMapFound {
                    if isMinigame {
                        ts.pszMapFile = UNKNOWN_MAP_NAME.to_string();
                    } else {
                        maplist.random(false);
                        ts.pszMapFile = maplist.current_shortmapname().to_string();
                    }
                } else {
                    ts.pszMapFile = maplist.current_shortmapname().to_string();
                }

                maplist.resume_current();
            }
        }

        if *version <= v(1, 8, 0, 2) && ts.iMode == 24 {
            ts.iMode = game_mode_pipe_minigame as i16;
        }

        if ts.iMode < 0 || (ts.iMode as i32 >= GAMEMODE_LAST && !isMinigame) {
            ts.iMode = RANDOM_INT(GAMEMODE_LAST) as i16;
        }

        pszTemp = st.tok(b",\n");

        ts.iGoal = -1;
        if let Some(p) = pszTemp {
            ts.iGoal = st.atoi(p) as i16;
        }

        if ts.iGoal <= 0 {
            if (ts.iMode as i32) < GAMEMODE_LAST {
                let idx = RANDOM_INT(GAMEMODE_NUM_OPTIONS as i32 - 1) as usize;
                ts.iGoal = unsafe { gamemodes[ts.iMode as usize].get_options()[idx].iValue };
            } else {
                ts.iGoal = 50;
            }
        }

        if *version >= v(1, 7, 0, 2) {
            pszTemp = st.tok(b",\n");

            ts.iPoints = match pszTemp {
                Some(p) => st.atoi(p) as i16,
                None => 1,
            };

            pszTemp = st.tok(b",\n");

            if fIsWorld {
                ts.iBonusType = 0;
                ts.iNumBonuses = 0;

                let mut pszStart = pszTemp;

                while let Some(start) = pszStart {
                    let pszEnd = st.strchr(start, b'|');
                    if let Some(e) = pszEnd {
                        st.buf[e] = 0;
                    }

                    let mut iWinnerPlace: i16 = st.at(start) as i16 - 48;
                    if iWinnerPlace == 0 {
                        break;
                    } else if !(1..=4).contains(&iWinnerPlace) {
                        iWinnerPlace = 1;
                    }

                    let b = &mut ts.wsbBonuses[ts.iNumBonuses as usize];
                    b.szBonusString = st.cstr(start);
                    b.iWinnerPlace = iWinnerPlace - 1;

                    let mut iPowerupOffset: i16 = 0;
                    if st.at(start + 1) == b'w' || st.at(start + 1) == b'W' {
                        iPowerupOffset += NUM_POWERUPS as i16;
                    }

                    let mut iBonus: i16 = (st.atoi(start + 2) + iPowerupOffset as i32) as i16;
                    if iBonus < 0 || iBonus as i32 >= NUM_POWERUPS + NUM_WORLD_POWERUPS {
                        iBonus = 0;
                    }

                    ts.wsbBonuses[ts.iNumBonuses as usize].iBonus = iBonus;

                    ts.iNumBonuses += 1;
                    if ts.iNumBonuses >= 10 {
                        break;
                    }

                    pszStart = pszEnd.map(|e| e + 1);
                }
            } else {
                ts.iBonusType = match pszTemp {
                    Some(p) => st.atoi(p) as i16,
                    None => 0,
                };
            }

            pszTemp = st.tok(b",\n");

            ts.szName = match pszTemp {
                Some(p) => st.cstr(p),
                None => format!("Tour Stop {}", unsafe { game_values.tourstops.len() } + 1),
            };
        } else {
            ts.iPoints = 1;
            ts.iBonusType = 0;
            ts.szName = format!("Tour Stop {}", unsafe { game_values.tourstops.len() } + 1);
        }

        if *version >= v(1, 8, 0, 0) {
            if fIsWorld {
                pszTemp = st.tok(b",\n");

                ts.fEndStage = match pszTemp {
                    Some(p) => st.at(p) == b'1',
                    None => false,
                };
            }

            ts.gmsSettings = unsafe { game_values.gamemodemenusettings.clone() };

            let mut gms = ts.gmsSettings.clone();
            let usedSettings = deserialize_gms(&mut st, ts.iMode, &mut gms, unsafe { &game_values.gamemodesettings });
            ts.gmsSettings = gms;
            ts.iNumUsedSettings = usedSettings.iter().filter(|&&b| b).count() as i16;
            ts.fUseSettings = !usedSettings.is_empty();
        }
    } else if ts.iStageType == 1 {
        ts.szName = match pszTemp {
            Some(p) => st.cstr(p),
            None => format!("Bonus House {}", unsafe { game_values.tourstops.len() } + 1),
        };

        pszTemp = st.tok(b",\n");

        let mut iBonusOrdering: i16 = st.atoi(pszTemp.expect("strtok returned NULL")) as i16;
        if !(0..=1).contains(&iBonusOrdering) {
            iBonusOrdering = 0;
        }

        ts.iBonusType = iBonusOrdering;

        pszTemp = st.tok(b",\n");

        let mut pszStart = pszTemp;

        ts.iBonusTextLines = 0;
        while let Some(start) = pszStart {
            if st.at(start) == b'-' {
                break;
            }
            let pszEnd = st.strchr(start, b'|');

            if let Some(e) = pszEnd {
                st.buf[e] = 0;
            }

            ts.szBonusText[ts.iBonusTextLines as usize] = st.cstr(start);

            ts.iBonusTextLines += 1;
            if ts.iBonusTextLines >= 5 || pszEnd.is_none() {
                break;
            }

            pszStart = pszEnd.map(|e| e + 1);
        }

        ts.iNumBonuses = 0;
        pszTemp = st.tok(b",\n");
        while let Some(p) = pszTemp {
            ts.wsbBonuses[ts.iNumBonuses as usize].szBonusString = st.cstr(p);

            let mut iPowerupOffset: i16 = 0;
            let c = st.at(p);
            if c == b'w' || c == b'W' {
                iPowerupOffset += NUM_POWERUPS as i16;
            } else if c == b's' || c == b'S' {
                iPowerupOffset += (NUM_POWERUPS + NUM_WORLD_POWERUPS - 1) as i16;
            }

            let mut iBonus: i16 = (st.atoi(p + 1) + iPowerupOffset as i32) as i16;
            if iBonus < 0 || iBonus as i32 >= NUM_POWERUPS + NUM_WORLD_POWERUPS + NUM_WORLD_SCORE_BONUSES {
                iBonus = 0;
            }

            ts.wsbBonuses[ts.iNumBonuses as usize].iBonus = iBonus;
            ts.wsbBonuses[ts.iNumBonuses as usize].iWinnerPlace = -1;

            ts.iNumBonuses += 1;
            if ts.iNumBonuses as i32 >= MAX_BONUS_CHESTS {
                break;
            }

            pszTemp = st.tok(b",\n");
        }
    }

    ts
}

pub fn write_tour_stop_line(ts: &TourStop, fIsWorld: bool) -> String {
    let mut fields: Vec<String> = Vec::new();

    if fIsWorld {
        fields.push(ts.iStageType.to_string());
    }

    if ts.iStageType == 0 {
        fields.push(ts.pszMapFile.clone());
        fields.push(ts.iMode.to_string());
        fields.push(ts.iGoal.to_string());
        fields.push(ts.iPoints.to_string());

        if fIsWorld {
            if ts.iNumBonuses <= 0 {
                fields.push("0".to_string());
            } else {
                let mut field = String::new();
                let mut delim = DELIM_EMPTY;
                for iBonus in 0..ts.iNumBonuses as usize {
                    field += delim;
                    field += &ts.wsbBonuses[iBonus].szBonusString;
                    delim = DELIM_PIPE;
                }
                fields.push(field);
            }
        } else {
            fields.push(ts.iBonusType.to_string());
        }

        fields.push(ts.szName.clone());

        if fIsWorld {
            fields.push((ts.fEndStage as i32).to_string());
        }

        if ts.fUseSettings {
            let mut values = serialize_gms(ts.iMode, &ts.gmsSettings);
            values.resize(ts.iNumUsedSettings.max(0) as usize, 0);
            for value in values {
                fields.push(value.to_string());
            }
        }
    } else if ts.iStageType == 1 {
        fields.push(ts.szName.clone());
        fields.push(ts.iBonusType.to_string());

        let mut bonus_text_field = String::new();
        let mut delim = DELIM_EMPTY;
        for iText in 0..ts.iBonusTextLines.max(0) as usize {
            bonus_text_field += delim;
            bonus_text_field += &ts.szBonusText[iText];
            delim = DELIM_PIPE;
        }
        fields.push(bonus_text_field);

        if ts.iNumBonuses == 0 {
            fields.push("p0".to_string());
        } else {
            for iBonus in 0..ts.iNumBonuses.max(0) as usize {
                fields.push(ts.wsbBonuses[iBonus].szBonusString.clone());
            }
        }
    }

    let mut out = String::new();
    let mut delim = DELIM_EMPTY;
    for field in fields {
        out += delim;
        out += &field;
        delim = DELIM_COMMA;
    }

    out += "\n";
    out
}

pub fn reset_tour_stops() {
    unsafe {
        game_values.tourstopcurrent = 0;

        if !game_values.tourstops.is_empty() {
            game_values.tourstops.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strtok_matches_c() {
        let mut st = Strtok::new(b",,a,b\n");
        let a = st.tok(b",\n").unwrap();
        assert_eq!(st.cstr(a), "a");
        let b = st.tok(b",\n").unwrap();
        assert_eq!(st.cstr(b), "b");
        assert!(st.tok(b",\n").is_none());
        assert!(st.tok(b",\n").is_none());
    }
}
