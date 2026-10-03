//! Port of src/smw/menu/options/SoundOptionsMenu.cpp

use crate::common::ui::menu_code::*;
use crate::common::ui::mi_button::MI_Button;
use crate::common::ui::mi_image::MI_Image;
use crate::common::ui::mi_select_field::MI_SelectField;
use crate::common::ui::mi_slider_field::MI_SliderField;
use crate::common::ui::mi_text::{MI_HeaderText, MI_Text};
use crate::common::uicontrol::{ctl_ptr, TextAlign};
use crate::common::uimenu::UI_Menu;
use crate::globals::*;
use crate::smw::menu::options::{select_field, OFF_ON};
use crate::smw::ui::mi_announcer_field::MI_AnnouncerField;
use crate::smw::ui::mi_packs_field::MI_PacksField;
use crate::smw::ui::mi_playlist_field::MI_PlaylistField;

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

#[derive(Default)]
pub struct UI_SoundOptionsMenu {
    pub ui_menu: UI_Menu,

    pub miSoundVolumeField: Ptr<MI_SliderField>,
    pub miMusicVolumeField: Ptr<MI_SliderField>,
    pub miPlayNextMusicField: Ptr<MI_SelectField<bool>>,
    pub miAnnouncerField: Ptr<MI_AnnouncerField>,
    pub miPlaylistField: Ptr<MI_PlaylistField>,
    pub miWorldMusicField: Ptr<MI_SelectField<i16>>,
    pub miSoundPackField: Ptr<MI_PacksField>,
    pub miSoundOptionsMenuBackButton: Ptr<MI_Button>,

    pub miSoundOptionsMenuLeftHeaderBar: Ptr<MI_Image>,
    pub miSoundOptionsMenuRightHeaderBar: Ptr<MI_Image>,
    pub miSoundOptionsMenuHeaderText: Ptr<MI_Text>,
}
crate::impl_base!(UI_SoundOptionsMenu => ui_menu: UI_Menu);

impl UI_SoundOptionsMenu {
    pub fn new() -> Box<Self> {
        let mut this = Box::<Self>::default();
        unsafe {
            let spr = Ptr::from_mut(&mut rm.spr_selectfield);
            let menu_plain_field = Ptr::from_mut(&mut rm.menu_plain_field);
            let menu_slider_bar = Ptr::from_mut(&mut rm.menu_slider_bar);

            let mut f = Ptr::new_box(MI_SliderField::new(spr, menu_slider_bar, 70, 100, "Sound Volume", 500, 220, 484));
            for &(name, value) in VOLUMES {
                f.add(name, value);
            }
            f.set_output_ptr(&mut game_values.soundvolume);
            f.set_current_value(game_values.soundvolume);
            f.allow_wrap(false);
            f.set_item_changed_code(MENU_CODE_SOUND_VOLUME_CHANGED);
            this.miSoundVolumeField = f;

            let mut f = Ptr::new_box(MI_SliderField::new(spr, menu_slider_bar, 70, 140, "Music Volume", 500, 220, 484));
            for &(name, value) in VOLUMES {
                f.add(name, value);
            }
            f.set_output_ptr(&mut game_values.musicvolume);
            f.set_current_value(game_values.musicvolume);
            f.allow_wrap(false);
            f.set_item_changed_code(MENU_CODE_MUSIC_VOLUME_CHANGED);
            this.miMusicVolumeField = f;

            this.miPlayNextMusicField = select_field(spr, 70, 180, "Next Music", 500, 220, OFF_ON, &mut game_values.playnextmusic);
            this.miPlayNextMusicField.set_auto_advance(true);

            this.miAnnouncerField = Ptr::new_box(MI_AnnouncerField::new(spr, 70, 220, "Announcer", 500, 220, Ptr::from_mut(&mut announcerlist.simple_file_list)));
            this.miSoundPackField = Ptr::new_box(MI_PacksField::new(spr, 70, 260, "Sound Pack", 500, 220, Ptr::from_mut(&mut soundpacklist.simple_directory_list.simple_file_list), MENU_CODE_SOUND_PACK_CHANGED));

            this.miPlaylistField = Ptr::new_box(MI_PlaylistField::new(spr, 70, 300, "Game Music Pack", 500, 220));
            this.miWorldMusicField = Ptr::new_box(MI_SelectField::new(spr, 70, 340, "World Music Pack", 500, 220));

            let iCurrentMusic = worldmusiclist.current_index() as i32;
            worldmusiclist.set_current(0);
            for iMusic in 0..worldmusiclist.count() {
                let name = worldmusiclist.current_name().to_string();
                this.miWorldMusicField.add(name, iMusic as i16);
                worldmusiclist.next();
            }
            this.miWorldMusicField.set_current_value(iCurrentMusic as i16);
            worldmusiclist.set_current(iCurrentMusic as usize);
            this.miWorldMusicField.set_item_changed_code(MENU_CODE_WORLD_MUSIC_CHANGED);

            this.miSoundOptionsMenuBackButton = Ptr::new_box(MI_Button::new(spr, 544, 432, "Back", 80, TextAlign::CENTER));
            this.miSoundOptionsMenuBackButton.set_code(MENU_CODE_BACK_TO_OPTIONS_MENU);

            this.miSoundOptionsMenuLeftHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 0, 0, 0, 0, 320, 32, 1, 1, 0));
            this.miSoundOptionsMenuRightHeaderBar = Ptr::new_box(MI_Image::new(menu_plain_field, 320, 0, 192, 0, 320, 32, 1, 1, 0));
            this.miSoundOptionsMenuHeaderText = Ptr::new_box(MI_HeaderText::new("Sound Options Menu", 320, 5));
        }

        let soundvol = ctl_ptr(this.miSoundVolumeField);
        let musicvol = ctl_ptr(this.miMusicVolumeField);
        let nextmusic = ctl_ptr(this.miPlayNextMusicField);
        let announcer = ctl_ptr(this.miAnnouncerField);
        let soundpack = ctl_ptr(this.miSoundPackField);
        let playlist = ctl_ptr(this.miPlaylistField);
        let worldmusic = ctl_ptr(this.miWorldMusicField);
        let back = ctl_ptr(this.miSoundOptionsMenuBackButton);
        let null = Ptr::null();

        this.add_control(soundvol, back, musicvol, null, back);
        this.add_control(musicvol, soundvol, nextmusic, null, back);
        this.add_control(nextmusic, musicvol, announcer, null, back);
        this.add_control(announcer, nextmusic, soundpack, null, back);
        this.add_control(soundpack, announcer, playlist, null, back);
        this.add_control(playlist, soundpack, worldmusic, null, back);
        this.add_control(worldmusic, playlist, back, null, back);
        this.add_control(back, worldmusic, soundvol, soundpack, null);

        let c = ctl_ptr(this.miSoundOptionsMenuLeftHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miSoundOptionsMenuRightHeaderBar);
        this.add_non_control(c);
        let c = ctl_ptr(this.miSoundOptionsMenuHeaderText);
        this.add_non_control(c);

        this.set_initial_focus(soundvol);
        this.set_cancel_code(MENU_CODE_BACK_TO_OPTIONS_MENU);
        this
    }

    pub fn get_current_world_music_id(&self) -> i16 {
        self.miWorldMusicField.current_value()
    }
}
