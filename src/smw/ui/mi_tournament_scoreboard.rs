//! Port of src/smw/ui/MI_TournamentScoreboard.cpp

use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::{MAX_WORLD_BONUSES_AWARDED, PGFX_STANDING_R, PI, TWO_PI, VELJUMP};
use crate::common::match_types::MatchType;
use crate::common::math::trig::{cos, sin};
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_score_text::MI_ScoreText;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;

const MWB: usize = MAX_WORLD_BONUSES_AWARDED as usize;

const iScoreboardPlayerOffsetsX: [[i16; 3]; 3] = [[40, 0, 0], [19, 59, 0], [6, 40, 74]];

pub struct MI_TournamentScoreboard {
    pub ui_control: UI_Control,

    fCreated: bool,

    miTeamImages: Vec<Box<MI_Image>>,
    miIconImages: Vec<Vec<Box<MI_Image>>>,
    miPlayerImages: Vec<Vec<Box<MI_Image>>>,

    iNumTeams: i16,
    iNumGames: i16,

    iTournamentWinner: i16,
    iGameWinner: i16,

    iSwirlIconTeam: i16,
    iSwirlIconGame: i16,

    iFireworksCounter: i16,
    iWinnerTextCounter: i16,
    iExplosionCounter: i16,

    sprBackground: Ptr<gfxSprite>,
    sprIcons: Ptr<gfxSprite>,

    iTeamIDs: [[i16; 3]; 4],
    iTeamCounts: [i16; 4],

    tourScores: [Option<Box<MI_ScoreText>>; 4],
    tourPoints: [Option<Box<MI_ScoreText>>; 10],
    miTourPointBar: Option<Box<MI_Image>>,
    tourBonus: [Option<Box<MI_Image>>; 10],

    worldBonus: [[Option<Box<MI_Image>>; MWB]; 4],
    worldScoreModifier: Option<Box<MI_Image>>,
    worldPlace: [Option<Box<MI_Image>>; 4],
    worldScore: Option<Box<MI_ScoreText>>,
    worldPointsBackground: [Option<Box<MI_Image>>; 4],
}
crate::impl_base!(MI_TournamentScoreboard => ui_control: UI_Control);

impl MI_TournamentScoreboard {
    //Call with x = 70 and y == 80
    pub fn new(spr_background: Ptr<gfxSprite>, x: i16, y: i16) -> Self {
        MI_TournamentScoreboard {
            ui_control: UI_Control::new(x, y),
            fCreated: false,
            miTeamImages: Vec::new(),
            miIconImages: Vec::new(),
            miPlayerImages: Vec::new(),
            iNumTeams: 0,
            iNumGames: 0,
            iTournamentWinner: 0,
            iGameWinner: 0,
            iSwirlIconTeam: 0,
            iSwirlIconGame: 0,
            iFireworksCounter: 0,
            iWinnerTextCounter: 0,
            iExplosionCounter: 0,
            sprBackground: spr_background,
            sprIcons: Ptr::null(),
            iTeamIDs: [[0; 3]; 4],
            iTeamCounts: [0; 4],
            tourScores: Default::default(),
            tourPoints: Default::default(),
            miTourPointBar: None,
            tourBonus: Default::default(),
            worldBonus: Default::default(),
            worldScoreModifier: None,
            worldPlace: Default::default(),
            worldScore: None,
            worldPointsBackground: Default::default(),
        }
    }

    fn get_y_from_place(&self, iPlace: i16) -> i16 {
        (self.m_pos.y as i32 + iPlace as i32 * 69 + (4 - self.iNumTeams as i32) * 35) as i16
    }

    fn get_icon_spacing(&self) -> f32 {
        (372 - self.iNumGames as i32 * 32) as f32 / (self.iNumGames as i32 + 1) as f32
    }

    fn free_scoreboard(&mut self) {
        if !self.fCreated {
            return;
        }

        self.miIconImages.clear();
        self.miPlayerImages.clear();
        self.miTeamImages.clear();

        self.tourScores = Default::default();
        self.worldPlace = Default::default();
        self.worldBonus = Default::default();
        self.worldPointsBackground = Default::default();

        self.tourPoints = Default::default();
        self.tourBonus = Default::default();

        self.miTourPointBar = None;
        self.worldScore = None;
        self.worldScoreModifier = None;
    }

    pub fn create_scoreboard(&mut self, numTeams: i16, numGames: i16, spr_icons_ref: Ptr<gfxSprite>) {
        self.free_scoreboard();

        self.sprIcons = spr_icons_ref;

        unsafe {
            for iTeam in 0..4usize {
                self.iTeamCounts[iTeam] = game_values.teamcounts[iTeam];

                for iTeamSpot in 0..3usize {
                    self.iTeamIDs[iTeam][iTeamSpot] = game_values.teamids[iTeam][iTeamSpot];
                }
            }

            self.iTournamentWinner = -1;
            self.iGameWinner = -1;

            self.iSwirlIconTeam = -1;
            self.iSwirlIconGame = -1;

            self.iExplosionCounter = 0;
            self.iFireworksCounter = 0;
            self.iWinnerTextCounter = 0;

            self.iNumTeams = numTeams;
            self.iNumGames = numGames;

            self.miTeamImages = Vec::with_capacity(self.iNumTeams as usize);
            self.miIconImages = Vec::with_capacity(self.iNumTeams as usize);
            self.miPlayerImages = Vec::with_capacity(self.iNumTeams as usize);

            let fTour = game_values.matchtype == MatchType::Tour;
            let fNotTournament = game_values.matchtype != MatchType::Tournament;

            let x = self.m_pos.x as i32;

            for iTeam in 0..self.iNumTeams {
                let t = iTeam as usize;
                let mut iTeamY = self.get_y_from_place(iTeam);

                if fTour {
                    iTeamY += 28; //shift down 28 pxls for extra tour points bar
                }

                self.miTeamImages.push(Box::new(MI_Image::new(
                    self.sprBackground,
                    (x - if fNotTournament { 40 } else { 0 }) as i16,
                    iTeamY,
                    0,
                    if game_values.matchtype == MatchType::World { 160 } else { 0 },
                    if fNotTournament { 580 } else { 500 },
                    64,
                    1,
                    2,
                    0,
                )));
                self.miIconImages.push(Vec::with_capacity(self.iNumGames.max(0) as usize));
                self.miPlayerImages.push(Vec::with_capacity(self.iTeamCounts[t].max(0) as usize));

                if game_values.matchtype != MatchType::World {
                    for iGame in 0..self.iNumGames {
                        let dSpacing = self.get_icon_spacing();

                        let mut icon = Box::new(MI_Image::new(
                            self.sprIcons,
                            (x + 128 + dSpacing as i16 as i32 + (iGame as f32 * (32.0f32 + dSpacing)) as i16 as i32 - if fTour { 40 } else { 0 }) as i16,
                            iTeamY + 16,
                            0,
                            0,
                            32,
                            32,
                            if fTour { 4 } else { 1 },
                            1,
                            if fTour { 8 } else { 0 },
                        ));
                        icon.set_visible(false);
                        self.miIconImages[t].push(icon);
                    }
                }

                for iPlayer in 0..self.iTeamCounts[t] {
                    let p = iPlayer as usize;
                    self.miPlayerImages[t].push(Box::new(MI_Image::new(
                        Ptr::from_mut(&mut rm.spr_player[self.iTeamIDs[t][p] as usize][PGFX_STANDING_R as usize]),
                        (x + iScoreboardPlayerOffsetsX[(self.iTeamCounts[t] - 1) as usize][p] as i32 - if fTour { 40 } else { 0 }) as i16,
                        iTeamY + 16,
                        0,
                        0,
                        32,
                        32,
                        2,
                        1,
                        0,
                    )));
                }

                if fNotTournament {
                    self.tourScores[t] = Some(Box::new(MI_ScoreText::new(0, 0)));
                }

                if game_values.matchtype == MatchType::World {
                    self.worldPointsBackground[t] = Some(Box::new(MI_Image::new(self.sprBackground, (x + 476) as i16, iTeamY, 516, 160, 64, 64, 1, 2, 0)));
                    self.worldPlace[t] = Some(Box::new(MI_Image::new(self.sprIcons, (x + 102) as i16, iTeamY + 14, 0, 0, 32, 32, 4, 1, 8)));

                    for iBonus in 0..MWB {
                        let mut bonus = Box::new(MI_Image::new(Ptr::from_mut(&mut rm.spr_worlditems), (x + 180 + 38 * iBonus as i32) as i16, iTeamY + 14, 0, 0, 32, 32, 1, 1, 0));
                        bonus.set_visible(false);
                        self.worldBonus[t][iBonus] = Some(bonus);
                    }
                }
            }

            if game_values.matchtype == MatchType::World {
                self.worldScoreModifier = Some(Box::new(MI_Image::new(Ptr::from_mut(&mut rm.spr_worlditems), 0, 0, 0, 0, 32, 32, 1, 1, 0)));
                self.worldScore = Some(Box::new(MI_ScoreText::new(0, 0)));
            }

            if fTour {
                for iGame in 0..self.iNumGames {
                    let dSpacing = self.get_icon_spacing();
                    let iTourPointX = (x + 105 + dSpacing as i16 as i32 + (iGame as f32 * (32.0f32 + dSpacing)) as i16 as i32) as i16;
                    let mut points = Box::new(MI_ScoreText::new(iTourPointX, self.get_y_from_place(0)));
                    // TODO(world_tour_stop): tourPoints[iGame]->SetScore(game_values.tourstops[iGame]->iPoints); and create tourBonus[iGame] when game_values.tourstops[iGame]->iBonusType
                    let _ = &mut points;
                    self.tourPoints[iGame as usize] = Some(points);
                    //else
                    //	tourBonus[iGame] = new MI_Image(sprBackground, iTourPointX - 11, GetYFromPlace(0) - 3, 448, 128, 22, 22, 1, 1, 0);
                }

                self.miTourPointBar = Some(Box::new(MI_Image::new(self.sprBackground, (x + 88) as i16, self.get_y_from_place(0) - 8, 0, 128, 372, 32, 1, 1, 0)));
            }
        }

        self.fCreated = true;
    }

    pub fn refresh_world_scores(&mut self, gameWinner: i16) {
        self.iGameWinner = gameWinner;
        self.determine_scoreboard_winners();

        // TODO(world_tour_stop): TourStop * tourStop = game_values.tourstops[game_values.tourstopcurrent];

        unsafe {
            let x = self.m_pos.x as i32;
            for iTeam in 0..self.iNumTeams {
                let t = iTeam as usize;
                let iTeamY = self.get_y_from_place(game_values.tournament_scores[t].wins);

                self.miTeamImages[t].set_position((x - 40) as i16, iTeamY);
                self.worldPointsBackground[t].as_mut().unwrap().set_position((x + 476) as i16, iTeamY);

                for iPlayer in 0..self.iTeamCounts[t] {
                    const iPlaceSprite: [i16; 4] = [4, 0, 8, 9];
                    let p = iPlayer as usize;
                    let img = &mut self.miPlayerImages[t][p];
                    img.set_position((x + iScoreboardPlayerOffsetsX[(self.iTeamCounts[t] - 1) as usize][p] as i32 - 40) as i16, iTeamY + 16);
                    img.set_image_source(Ptr::from_mut(
                        &mut rm.spr_player[self.iTeamIDs[t][p] as usize][iPlaceSprite[game_values.tournament_scores[t].wins as usize] as usize],
                    ));
                }

                let ts = self.tourScores[t].as_mut().unwrap();
                ts.set_position((x + 508) as i16, iTeamY + 24);
                ts.set_score(game_values.tournament_scores[t].total);

                if self.iGameWinner == -1 {
                    self.worldScore.as_mut().unwrap().set_visible(false);
                    self.worldScoreModifier.as_mut().unwrap().set_visible(false);
                }

                if self.iGameWinner == iTeam {
                    let ws = self.worldScore.as_mut().unwrap();
                    ws.set_position((x + 350) as i16, iTeamY + 24);
                    // TODO(world_tour_stop): worldScore->SetScore(tourStop->iPoints);
                    ws.set_visible(true);

                    self.miTeamImages[t].set_image(0, 160, 496, 64);

                    let wsm = self.worldScoreModifier.as_mut().unwrap();
                    if game_values.worldpointsbonus >= 0 {
                        wsm.set_image((game_values.worldpointsbonus + 9) << 5, 0, 32, 32);
                        wsm.set_position((x + 410) as i16, iTeamY + 14);
                        wsm.set_visible(true);
                    } else {
                        wsm.set_visible(false);
                    }
                } else {
                    self.miTeamImages[t].set_image(0, 160, 344, 64);
                }

                let wp = self.worldPlace[t].as_mut().unwrap();
                wp.set_position((x + 102) as i16, iTeamY + 14);
                wp.set_image(0, crate::smw::main::score[t].place << 5, 32, 32);
                wp.set_visible(gameWinner >= 0);
            }

            let iBonusCounts: [i16; 4] = [0, 0, 0, 0];

            if gameWinner >= 0 {
                // TODO(world_tour_stop): award loop over tourStop->wsbBonuses[0..iNumBonuses] (shows worldBonus images, increments iBonusCounts).
            }

            for iTeam in 0..self.iNumTeams as usize {
                for iBonus in iBonusCounts[iTeam] as usize..MWB {
                    self.worldBonus[iTeam][iBonus].as_mut().unwrap().set_visible(false);
                }
            }

            if gameWinner >= 0 {
                game_values.worldpointsbonus = -1;
            }
        }
    }

    //Called by Tour -- Arranges players in terms of standings
    pub fn refresh_tour_scores(&mut self) {
        self.determine_scoreboard_winners();

        unsafe {
            let x = self.m_pos.x as i32;
            for iTeam in 0..self.iNumTeams {
                let t = iTeam as usize;
                let mut iTeamY = self.get_y_from_place(game_values.tournament_scores[t].wins);
                iTeamY += 28; //shift down 28 pxls for extra tour points bar

                let dSpacing = self.get_icon_spacing();

                self.miTeamImages[t].set_position((x - 40) as i16, iTeamY);

                for iPlayer in 0..self.iTeamCounts[t] {
                    const iPlaceSprite: [i16; 4] = [4, 0, 8, 9];
                    let p = iPlayer as usize;
                    let img = &mut self.miPlayerImages[t][p];
                    img.set_position((x + iScoreboardPlayerOffsetsX[(self.iTeamCounts[t] - 1) as usize][p] as i32 - 40) as i16, iTeamY + 16);
                    img.set_image_source(Ptr::from_mut(
                        &mut rm.spr_player[self.iTeamIDs[t][p] as usize][iPlaceSprite[game_values.tournament_scores[t].wins as usize] as usize],
                    ));
                }

                for iGame in 0..game_values.tourstopcurrent {
                    let icon = &mut self.miIconImages[t][iGame];
                    icon.set_image(0, game_values.tournament_scores[t].type_[iGame] * 32, 32, 32);

                    icon.set_position(
                        (x + 128 + dSpacing as i16 as i32 + (iGame as i16 as f32 * (32.0f32 + dSpacing)) as i16 as i32 - 40) as i16,
                        iTeamY + 16,
                    );

                    icon.set_swirl(false, 0.0, 0.0, 0.0, 0.0);
                    icon.set_pulse(false);
                    icon.set_visible(true);
                }

                // TODO(world_tour_stop): C++ upper bound is game_values.tourstops.size(); CreateScoreboard sized the row with it.
                for iGame in game_values.tourstopcurrent..self.iNumGames as usize {
                    self.miIconImages[t][iGame].set_visible(false);
                }

                let ts = self.tourScores[t].as_mut().unwrap();
                ts.set_position((x + 508) as i16, iTeamY + 24);
                ts.set_score(game_values.tournament_scores[t].total);
            }
        }
    }

    pub fn determine_scoreboard_winners(&mut self) {
        unsafe {
            //Detect a Tie
            let mut iNumWinningTeams: i16 = 0;
            let mut iWinningTeams: [i16; 4] = [-1, -1, -1, -1];

            for iTeam in 0..self.iNumTeams {
                if game_values.tournament_scores[iTeam as usize].wins == 0 {
                    iWinningTeams[iNumWinningTeams as usize] = iTeam;
                    iNumWinningTeams += 1;
                }
            }

            //Adjust tied scores so display is right
            for iMyTeam in 0..self.iNumTeams as usize {
                for iTheirTeam in 0..self.iNumTeams as usize {
                    if iMyTeam == iTheirTeam {
                        continue;
                    }

                    if game_values.tournament_scores[iMyTeam].wins == game_values.tournament_scores[iTheirTeam].wins {
                        game_values.tournament_scores[iTheirTeam].wins += 1;
                    }
                }
            }

            //There was a single team winner
            if game_values.tournamentwinner != -1 {
                if iNumWinningTeams == 1 {
                    self.iTournamentWinner = iWinningTeams[0];
                } else {
                    self.iTournamentWinner = -2;
                }

                game_values.tournamentwinner = self.iTournamentWinner;

                //Flash the background of the winning teams
                for iTeam in 0..iNumWinningTeams as usize {
                    let w = iWinningTeams[iTeam] as usize;
                    self.miTeamImages[w].set_animation_speed(20);

                    if let Some(bg) = self.worldPointsBackground[w].as_mut() {
                        bg.set_animation_speed(20);
                    }
                }
            }
        }
    }

    //Called by Tournament -- Keeps players where they are and displays number of wins and mode type of win
    pub fn refresh_tournament_scores(&mut self, gameWinner: i16) {
        self.iGameWinner = gameWinner;
        self.iSwirlIconTeam = -1;
        self.iSwirlIconGame = -1;

        unsafe {
            if game_values.tournament_scores[self.iGameWinner as usize].wins == self.iNumGames {
                self.iTournamentWinner = self.iGameWinner;
                let w = self.iTournamentWinner as usize;
                self.miTeamImages[w].set_animation_speed(20);

                if let Some(bg) = self.worldPointsBackground[w].as_mut() {
                    bg.set_animation_speed(20);
                }
            }

            for iTeam in 0..self.iNumTeams {
                let t = iTeam as usize;
                for iGame in 0..game_values.tournament_scores[t].wins {
                    let g = iGame as usize;
                    self.miIconImages[t][g].set_image(game_values.tournament_scores[t].type_[g] * 32, 0, 32, 32);

                    if self.iTournamentWinner < 0 && self.iGameWinner == iTeam && iGame == game_values.tournament_scores[t].wins - 1 {
                        self.iSwirlIconTeam = iTeam;
                        self.iSwirlIconGame = iGame;

                        let angle = RANDOM_INT(1000) as f32 * TWO_PI / 1000.0f32;
                        self.miIconImages[t][g].set_swirl(true, 250.0, angle, 3.0, 0.1);
                    }

                    self.miIconImages[t][g].set_pulse(false);
                    self.miIconImages[t][g].set_visible(true);
                }

                for iGame in game_values.tournament_scores[t].wins..self.iNumGames {
                    self.miIconImages[t][iGame as usize].set_visible(false);
                }
            }
        }
    }

    pub fn stop_swirl(&mut self) {
        if self.iSwirlIconTeam >= 0 {
            self.miIconImages[self.iSwirlIconTeam as usize][self.iSwirlIconGame as usize].stop_swirl();
            self.iSwirlIconTeam = -1;
            self.iSwirlIconGame = -1;
        }
    }
}

impl UI_ControlTrait for MI_TournamentScoreboard {
    crate::impl_ctl!();

    fn update(&mut self) {
        unsafe {
            for iTeam in 0..self.iNumTeams as usize {
                self.miTeamImages[iTeam].update();

                for iGame in 0..self.iNumGames as usize {
                    self.miIconImages[iTeam][iGame].update();
                }

                for iPlayer in 0..self.iTeamCounts[iTeam] as usize {
                    self.miPlayerImages[iTeam][iPlayer].update();
                }

                if let Some(bg) = self.worldPointsBackground[iTeam].as_mut() {
                    bg.update();
                }
            }

            if game_values.matchtype == MatchType::Tour {
                for iGame in 0..self.iNumGames as usize {
                    if let Some(b) = self.tourBonus[iGame].as_mut() {
                        b.update();
                    }
                }
            }

            if !self.m_parentMenu.is_null() {
                if self.iTournamentWinner != -1 {
                    //Single tournament winning team
                    self.iFireworksCounter -= 1;
                    if self.iFireworksCounter < 0 && self.iTournamentWinner >= 0 {
                        self.iFireworksCounter = (RANDOM_INT(30) + 10) as i16;

                        let w = self.iTournamentWinner as usize;
                        self.iExplosionCounter -= 1;
                        if self.iExplosionCounter < 0 {
                            self.iExplosionCounter = (RANDOM_INT(6) + 5) as i16;

                            if_sound_on_play(&mut rm.sfx_bobombsound);

                            let mut dAngle: f32 = 0.0;
                            let iRandX = (RANDOM_INT(440) + 100) as i16;
                            let iRandY = (RANDOM_INT(280) + 100) as i16;

                            for iBlock in 0..28i16 {
                                let dVel: f32 = 7.0f32 + ((iBlock % 2) as f32 * 5.0f32);
                                let dVelX = dVel * cos(dAngle);
                                let dVelY = dVel * sin(dAngle);

                                let iRandomColor = RANDOM_INT(self.iTeamCounts[w] as i32) as i16;
                                let _ = (iRandX, iRandY, dVelX, dVelY, game_values.colorids[self.iTeamIDs[w][iRandomColor as usize] as usize] << 4);
                                // TODO(eyecandy): m_parentMenu->AddEyeCandy<EC_FallingObject>(&rm->spr_bonus, iRandX, iRandY, dVelX, dVelY, 4, 2, 0, game_values.colorids[iTeamIDs[iTournamentWinner][iRandomColor]] << 4, 16, 16);
                                dAngle -= PI / 14.0f32;
                            }
                        } else {
                            if_sound_on_play(&mut rm.sfx_cannon);

                            let iRandX = RANDOM_INT(576) as i16;
                            let iRandY = RANDOM_INT(416) as i16;
                            let iRandomColor = RANDOM_INT(self.iTeamCounts[w] as i32) as i16;

                            let _ = (iRandX, iRandY, game_values.colorids[self.iTeamIDs[w][iRandomColor as usize] as usize] << 6);
                            // TODO(eyecandy): m_parentMenu->AddEyeCandy<EC_SingleAnimation>(&rm->spr_fireworks, iRandX, iRandY, 8, 4, 0, game_values.colorids[iTeamIDs[iTournamentWinner][iRandomColor]] << 6, 64, 64);
                        }
                    }

                    self.iWinnerTextCounter -= 1;
                    if self.iWinnerTextCounter < 0 {
                        self.iWinnerTextCounter = (RANDOM_INT(35) + 15) as i16;

                        let mut szWinnerText = String::new();
                        if self.iTournamentWinner == -2 {
                            szWinnerText = "Tied Game!".to_string();
                        } else if self.iTeamCounts[self.iTournamentWinner as usize] == 1 {
                            szWinnerText = format!("Player {} Wins!", self.iTeamIDs[self.iTournamentWinner as usize][0] + 1);
                        } else if self.iTeamCounts[self.iTournamentWinner as usize] > 1 {
                            szWinnerText = format!("Team {} Wins!", self.iTournamentWinner + 1);
                        }

                        let iStringWidth = rm.menu_font_large.get_width(&szWinnerText) as i16;
                        let iRandX = (RANDOM_INT(App::screenWidth - iStringWidth as i32) + (iStringWidth >> 1) as i32) as i16;
                        let iRandY = (RANDOM_INT(App::screenHeight - 100) + 100) as i16;

                        let _ = (iRandX, iRandY, -VELJUMP);
                        // TODO(eyecandy): m_parentMenu->AddEyeCandy<EC_GravText>(&rm->menu_font_large, iRandX, iRandY, szWinnerText, -VELJUMP);
                    }
                }
            }
        }
    }

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        for iTeam in 0..self.iNumTeams {
            let t = iTeam as usize;
            self.miTeamImages[t].draw();

            for iGame in 0..self.iNumGames {
                if self.iSwirlIconTeam != iTeam || self.iSwirlIconGame != iGame {
                    self.miIconImages[t][iGame as usize].draw();
                }
            }

            for iPlayer in 0..self.iTeamCounts[t] as usize {
                self.miPlayerImages[t][iPlayer].draw();
            }
        }

        if self.iSwirlIconTeam > -1 {
            self.miIconImages[self.iSwirlIconTeam as usize][self.iSwirlIconGame as usize].draw();
        }

        unsafe {
            //Draw tour totals
            if game_values.matchtype == MatchType::Tour {
                for iTeam in 0..self.iNumTeams as usize {
                    self.tourScores[iTeam].as_mut().unwrap().draw();
                }

                self.miTourPointBar.as_mut().unwrap().draw();

                for iGame in 0..self.iNumGames as usize {
                    if let Some(b) = self.tourBonus[iGame].as_mut() {
                        b.draw();
                    }

                    self.tourPoints[iGame].as_mut().unwrap().draw();
                }
            }

            if game_values.matchtype == MatchType::World {
                for iTeam in 0..self.iNumTeams as usize {
                    self.worldPointsBackground[iTeam].as_mut().unwrap().draw();
                }

                for iTeam in 0..self.iNumTeams as usize {
                    self.tourScores[iTeam].as_mut().unwrap().draw();
                    self.worldPlace[iTeam].as_mut().unwrap().draw();

                    for iBonus in 0..MWB {
                        self.worldBonus[iTeam][iBonus].as_mut().unwrap().draw();
                    }
                }

                self.worldScoreModifier.as_mut().unwrap().draw();
                self.worldScore.as_mut().unwrap().draw();
            }
        }
    }
}
