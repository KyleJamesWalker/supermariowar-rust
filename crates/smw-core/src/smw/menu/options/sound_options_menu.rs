//! Music and sound options: volumes, next music, announcer, sound pack and music playlists.

use crate::common::uicontrol::{ControlId, Ctl};
use crate::game::Game;
use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_slider_field::MI_SliderField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::menu::options::{select_field, OFF_ON};
use crate::smw::ui::mi_announcer_field::MI_AnnouncerField;
use crate::smw::ui::mi_packs_field::MI_PacksField;
use crate::smw::ui::mi_playlist_field::MI_PlaylistField;

/// Volume steps on a 0 to 128 scale.
const VOLUMES: &[(&str, i16)] = &[
    ("Off", 0),
    ("1", 16),
    ("2", 32),
    ("3", 48),
    ("4", 64),
    ("5", 80),
    ("6", 96),
    ("7", 112),
    ("Max", 128),
];

/// Volume sliders, pack and playlist pickers for music and sound.
#[derive(Default)]
pub struct UI_SoundOptionsMenu {
    pub ui_menu: UI_Menu,

    pub miSoundVolumeField: Ctl<MI_SliderField>,
    pub miMusicVolumeField: Ctl<MI_SliderField>,
    pub miPlayNextMusicField: Ctl<MI_SelectField<bool>>,
    pub miAnnouncerField: Ctl<MI_AnnouncerField>,
    pub miPlaylistField: Ctl<MI_PlaylistField>,
    /// Index into the world music list, applied on `MENU_CODE_WORLD_MUSIC_CHANGED`.
    pub miWorldMusicField: Ctl<MI_SelectField<i16>>,
    pub miSoundPackField: Ctl<MI_PacksField>,
    pub miSoundOptionsMenuBackButton: Ctl<MI_Button>,

    pub miSoundOptionsMenuLeftHeaderBar: Ctl<MI_Image>,
    pub miSoundOptionsMenuRightHeaderBar: Ctl<MI_Image>,
    pub miSoundOptionsMenuHeaderText: Ctl<MI_Text>,
}
crate::impl_base!(UI_SoundOptionsMenu => ui_menu: UI_Menu);

impl UI_SoundOptionsMenu {
    pub fn new(g: &mut Game) -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr = g.rm.spr_selectfield;
            let menu_plain_field = g.rm.menu_plain_field;
            let menu_slider_bar = g.rm.menu_slider_bar;

            let mut f = Ctl::new(MI_SliderField::new(spr, menu_slider_bar, 70, 100, "Sound Volume", 500, 220, 484), g);
            for &(name, value) in VOLUMES {
                g.ctl(f).add(name, value);
            }
            g.ctl(f).set_output_ptr(&mut g.game_values.soundvolume);
            g.ctl(f).set_current_value(g.game_values.soundvolume);
            g.ctl(f).allow_wrap(false);
            g.ctl(f).set_item_changed_code(MENU_CODE_SOUND_VOLUME_CHANGED);
            this.miSoundVolumeField = f;

            let mut f = Ctl::new(MI_SliderField::new(spr, menu_slider_bar, 70, 140, "Music Volume", 500, 220, 484), g);
            for &(name, value) in VOLUMES {
                g.ctl(f).add(name, value);
            }
            g.ctl(f).set_output_ptr(&mut g.game_values.musicvolume);
            g.ctl(f).set_current_value(g.game_values.musicvolume);
            g.ctl(f).allow_wrap(false);
            g.ctl(f).set_item_changed_code(MENU_CODE_MUSIC_VOLUME_CHANGED);
            this.miMusicVolumeField = f;

            this.miPlayNextMusicField = select_field(spr, 70, 180, "Next Music", 500, 220, OFF_ON, &mut g.game_values.playnextmusic, g);
            g.ctl(this.miPlayNextMusicField).set_auto_advance(true);

            this.miAnnouncerField = Ctl::new(MI_AnnouncerField::new(spr, 70, 220, "Announcer", 500, 220, Ptr::from_mut(&mut g.announcerlist.simple_file_list)), g);
            this.miSoundPackField = Ctl::new(MI_PacksField::new(spr, 70, 260, "Sound Pack", 500, 220, Ptr::from_mut(&mut g.soundpacklist.simple_directory_list.simple_file_list), MENU_CODE_SOUND_PACK_CHANGED), g);

            this.miPlaylistField = Ctl::new(MI_PlaylistField::new(spr, 70, 300, "Game Music Pack", 500, 220), g);
            this.miWorldMusicField = Ctl::new(MI_SelectField::new(spr, 70, 340, "World Music Pack", 500, 220), g);

            let iCurrentMusic = g.worldmusiclist.current_index() as i32;
            g.worldmusiclist.set_current(0);
            for iMusic in 0..g.worldmusiclist.count() {
                let name = g.worldmusiclist.current_name().to_string();
                g.ctl(this.miWorldMusicField).add(name, iMusic as i16);
                g.worldmusiclist.next();
            }
            if g.worldmusiclist.count() == 0 {
                g.ctl(this.miWorldMusicField).add("(none)".to_string(), 0);
            }
            g.ctl(this.miWorldMusicField).set_current_value(iCurrentMusic as i16);
            g.worldmusiclist.set_current(iCurrentMusic as usize);
            g.ctl(this.miWorldMusicField).set_item_changed_code(MENU_CODE_WORLD_MUSIC_CHANGED);

            this.miSoundOptionsMenuBackButton = Ctl::new(MI_Button::new(g, spr, 544, 432, "Back", 80, TextAlign::CENTER), g);
            g.ctl(this.miSoundOptionsMenuBackButton).set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            this.miSoundOptionsMenuLeftHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0), g);
            this.miSoundOptionsMenuRightHeaderBar = Ctl::new(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0), g);
            this.miSoundOptionsMenuHeaderText = Ctl::new(MI_HeaderText::new(g, "Sound Options Menu", 320, 5), g);
        }

        let soundvol = this.miSoundVolumeField.id();
        let musicvol = this.miMusicVolumeField.id();
        let nextmusic = this.miPlayNextMusicField.id();
        let announcer = this.miAnnouncerField.id();
        let soundpack = this.miSoundPackField.id();
        let playlist = this.miPlaylistField.id();
        let worldmusic = this.miWorldMusicField.id();
        let back = this.miSoundOptionsMenuBackButton.id();
        let null = ControlId::NONE;

        this.add_control(g, soundvol, back, musicvol, null, back);
        this.add_control(g, musicvol, soundvol, nextmusic, null, back);
        this.add_control(g, nextmusic, musicvol, announcer, null, back);
        this.add_control(g, announcer, nextmusic, soundpack, null, back);
        this.add_control(g, soundpack, announcer, playlist, null, back);
        this.add_control(g, playlist, soundpack, worldmusic, null, back);
        this.add_control(g, worldmusic, playlist, back, null, back);
        this.add_control(g, back, worldmusic, soundvol, soundpack, null);

        let c = this.miSoundOptionsMenuLeftHeaderBar.id();
        this.add_non_control(c);
        let c = this.miSoundOptionsMenuRightHeaderBar.id();
        this.add_non_control(c);
        let c = this.miSoundOptionsMenuHeaderText.id();
        this.add_non_control(c);

        this.set_initial_focus(g, soundvol);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }

    /// The selected index into the world music list.
    pub fn get_current_world_music_id(&self, g: &Game) -> i16 {
        g.ctl(self.miWorldMusicField).current_value()
    }
}
