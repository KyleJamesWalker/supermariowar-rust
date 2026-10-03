//! Port of src/common/TilesetManager.cpp

use crate::common::file_io::BinaryFile;
use crate::common::gfx::gfx_sprite::{gfxSprite, ImageLoader};
use crate::common::global_constants::*;
use crate::common::path::{convert_path, convert_path_pack};
use crate::common::tile_types::*;
use crate::common::util::container_helpers;
use crate::common::util::dir_iterator::SubdirsIterator;
use crate::common::util::sdl_helpers::SdlSurfacePtr;
use crate::globals::{Aliased, Ptr};
use sdl2::sys::{SDL_CreateRGBSurfaceWithFormat, SDL_GetError, SDL_Rect, SDL_Surface, SDL_UpperBlit};
use std::collections::HashSet;
use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;

// The map format supports tilesets with up to 128x128 tiles
pub const MAX_TILES_PER_AXIS: i32 = 128;
pub const MAX_TILES: i32 = MAX_TILES_PER_AXIS * MAX_TILES_PER_AXIS;

fn generate_tileset_rects(tilesize: i32) -> Vec<SDL_Rect> {
    let mut rects = Vec::with_capacity(MAX_TILES as usize);
    for row in 0..MAX_TILES_PER_AXIS {
        for col in 0..MAX_TILES_PER_AXIS {
            rects.push(SDL_Rect { x: col * tilesize, y: row * tilesize, w: tilesize, h: tilesize });
        }
    }
    rects
}

fn read_tile_type_file(path: &Path) -> Vec<TileType> {
    let path_str = path.to_string_lossy().into_owned();
    println!("Reading {}", path_str);
    //Detect if the tiletype file already exists, if not create it
    if !path.exists() {
        println!("Not found, will use blank tiles");
        return Vec::new();
    }

    let mut tsf = BinaryFile::new(&path_str, "rb");
    if !tsf.is_open() {
        println!("ERROR: couldn't open tileset file: {}", path_str);
        return Vec::new();
    }

    let tiletype_count = tsf.read_i32();
    if tiletype_count <= 0 || tiletype_count > MAX_TILES {
        return Vec::new();
    }

    let mut tiletypes = Vec::with_capacity(tiletype_count as usize);
    for _ in 0..tiletype_count {
        tiletypes.push(TileType::from_i32(tsf.read_i32()));
    }

    tiletypes
}

fn load_image_or_downscale(path: &Path, largeRes: &gfxSprite) -> gfxSprite {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ImageLoader::new(path).create())) {
        Ok(sprite) => sprite,
        Err(err) => {
            let msg = err.downcast_ref::<String>().cloned().unwrap_or_default();
            println!("\nwarning: {} -> falling back to downscaled image", msg);

            unsafe {
                let large = largeRes.get_surface();
                let format = (*large).format;
                let surf = SdlSurfacePtr::new(SDL_CreateRGBSurfaceWithFormat(
                    0,
                    largeRes.get_width() / 2,
                    largeRes.get_height() / 2,
                    (*format).BitsPerPixel as i32,
                    (*format).format,
                ));
                if SDL_UpperBlit(large, null_mut(), surf.get(), null_mut()) < 0 {
                    eprintln!("SDL_BlitSurface error: {}", CStr::from_ptr(SDL_GetError()).to_string_lossy());
                }

                gfxSprite::from_surface(surf, None)
            }
        }
    }
}

/// `std::filesystem::path::filename()`: empty for a path ending in a separator.
fn path_filename(path: &str) -> String {
    if path.ends_with('/') {
        return String::new();
    }
    Path::new(path).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default()
}

/*********************************
*  CTileset
*********************************/

pub struct CTileset {
    m_name: String,
    m_tileset_dir: PathBuf,
    m_tiletypes: Vec<TileType>,

    m_sprite_large: gfxSprite,
    m_sprite_medium: gfxSprite,
    m_sprite_small: gfxSprite,

    m_width: i16,
    m_height: i16,
    m_modified: bool,
    pub _alias: Aliased,
}

impl CTileset {
    pub fn new(dir: PathBuf) -> Self {
        CTileset {
            m_name: dir.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default(),
            m_tileset_dir: dir,
            m_tiletypes: Vec::new(),
            m_sprite_large: gfxSprite::new(),
            m_sprite_medium: gfxSprite::new(),
            m_sprite_small: gfxSprite::new(),
            m_width: 0,
            m_height: 0,
            m_modified: false,
            _alias: Aliased::new(),
        }
    }

    pub fn ensure_loaded(&mut self) {
        if self.is_loaded() {
            return;
        }

        self.m_tiletypes = read_tile_type_file(&self.m_tileset_dir.join("tileset.tls"));

        self.m_sprite_large = ImageLoader::new(self.m_tileset_dir.join("large.png")).create();
        self.m_sprite_medium = load_image_or_downscale(&self.m_tileset_dir.join("medium.png"), &self.m_sprite_large);
        self.m_sprite_small = load_image_or_downscale(&self.m_tileset_dir.join("small.png"), &self.m_sprite_medium);

        self.m_width = (self.m_sprite_large.get_width() / TILESIZE).min(MAX_TILES_PER_AXIS) as i16;
        self.m_height = (self.m_sprite_large.get_height() / TILESIZE).min(MAX_TILES_PER_AXIS) as i16;
    }

    pub fn is_loaded(&self) -> bool {
        !self.m_sprite_large.get_surface().is_null()
    }

    pub fn is_modified(&self) -> bool {
        self.m_modified
    }

    /// `sprite(DrawSize)`: 0 ingame, 1 preview, 2 thumbnail.
    pub fn sprite(&self, size: i16) -> &gfxSprite {
        debug_assert!(self.is_loaded()); // call ensureLoaded first!
        match size {
            1 => &self.m_sprite_medium,
            2 => &self.m_sprite_small,
            _ => &self.m_sprite_large,
        }
    }

    pub fn surface(&self, size: usize) -> *mut SDL_Surface {
        self.sprite(size as i16).get_surface()
    }

    pub fn tile_type(&self, tileCol: usize, tileRow: usize) -> TileType {
        let idx = tileCol.wrapping_add(tileRow.wrapping_mul(self.m_width as usize));
        self.m_tiletypes.get(idx).copied().unwrap_or(TileType::NonSolid)
    }

    pub fn set_tile_type(&mut self, tileCol: usize, tileRow: usize, r#type: TileType) {
        let idx = tileCol + tileRow * self.m_width as usize;
        if self.m_tiletypes.len() <= idx {
            self.m_tiletypes.resize(idx + 1, TileType::NonSolid);
        }
        self.m_tiletypes[idx] = r#type;
        self.m_modified = true;
    }

    pub fn increment_tile_type(&mut self, tileCol: usize, tileRow: usize) -> TileType {
        let new_type = next_tile_type(self.tile_type(tileCol, tileRow));
        self.set_tile_type(tileCol, tileRow, new_type);
        new_type
    }

    pub fn decrement_tile_type(&mut self, tileCol: usize, tileRow: usize) -> TileType {
        let new_type = prev_tile_type(self.tile_type(tileCol, tileRow));
        self.set_tile_type(tileCol, tileRow, new_type);
        new_type
    }

    pub fn draw(&self, dstSurface: *mut SDL_Surface, tileSize: i16, srcRect: *mut SDL_Rect, dstRect: *mut SDL_Rect) {
        unsafe {
            SDL_UpperBlit(self.surface(tileSize as usize), srcRect, dstSurface, dstRect);
        }
    }

    pub fn save_tileset(&self) {
        debug_assert!(self.is_loaded());
        if !self.is_loaded() {
            return;
        }

        let tileset_path = self.m_tileset_dir.join("tileset.tls").to_string_lossy().into_owned();
        let mut tsf = BinaryFile::new(&tileset_path, "wb");
        if !tsf.is_open() {
            println!("ERROR: couldn't open tileset file to save tile types: {}", tileset_path);
            return;
        }

        tsf.write_i32(self.m_tiletypes.len() as i32);

        for tiletype in &self.m_tiletypes {
            tsf.write_i32(tiletype.0 as i32);
        }
        drop(tsf);

        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tileset_path, std::fs::Permissions::from_mode(0o774));
    }

    pub fn name(&self) -> &str {
        &self.m_name
    }
    pub fn height(&self) -> i16 {
        self.m_height
    }
    pub fn width(&self) -> i16 {
        self.m_width
    }
}

/*********************************
*  CTilesetManager
*********************************/

pub struct CTilesetManager {
    m_tilesets: Vec<Box<CTileset>>,
    m_classicTilesetIndex: usize,

    m_rects_ingame: Vec<SDL_Rect>,
    m_rects_preview: Vec<SDL_Rect>,
    m_rects_thumb: Vec<SDL_Rect>,

    pub _alias: Aliased,
}

impl CTilesetManager {
    pub fn new(gfxPack: &str) -> Self {
        let mut m_tilesets: Vec<Box<CTileset>> = Vec::new();
        let mut found_tileset_names: HashSet<String> = HashSet::new();

        let mut dir = SubdirsIterator::new(convert_path_pack("gfx/packs/tilesets", gfxPack) + "/");
        while let Some(path) = dir.next() {
            let tileset = Box::new(CTileset::new(path));
            found_tileset_names.insert(tileset.name().to_string());
            m_tilesets.push(tileset);
        }

        //Add tilesets from the Classic pack to fill the gaps
        if path_filename(gfxPack) != "Classic" {
            dir = SubdirsIterator::new(convert_path("gfx/packs/Classic/tilesets/"));
            while let Some(path) = dir.next() {
                let name = path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                if !found_tileset_names.contains(&name) {
                    m_tilesets.push(Box::new(CTileset::new(path)));
                }
            }
        }

        container_helpers::sort(&mut m_tilesets, |a, b| a.name().as_bytes() < b.name().as_bytes());

        let m_classicTilesetIndex = m_tilesets.iter().position(|t| t.name() == "Classic").unwrap_or(usize::MAX);

        CTilesetManager {
            m_tilesets,
            m_classicTilesetIndex,
            m_rects_ingame: generate_tileset_rects(TILESIZE),
            m_rects_preview: generate_tileset_rects(PREVIEWTILESIZE),
            m_rects_thumb: generate_tileset_rects(THUMBTILESIZE),
            _alias: Aliased::new(),
        }
    }

    /// Returns `TILESETUNKNOWN` converted to `size_t` when not found, like C++.
    pub fn index_from_name(&self, name: &str) -> usize {
        for (i, t) in self.m_tilesets.iter().enumerate() {
            if t.name() == name {
                return i;
            }
        }

        TILESETUNKNOWN as isize as usize
    }

    pub fn count(&self) -> usize {
        self.m_tilesets.len()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        dstSurface: *mut SDL_Surface,
        iTilesetID: i16,
        iTileSize: i16,
        iSrcTileCol: i16,
        iSrcTileRow: i16,
        iDstTileCol: i16,
        iDstTileRow: i16,
    ) {
        let tileset_ptr = self.tileset(iTilesetID as isize as usize);
        if tileset_ptr.is_null() {
            return;
        }

        let src_rect = self.rect(iTileSize, iSrcTileCol, iSrcTileRow);
        let dst_rect = self.rect(iTileSize, iDstTileCol, iDstTileRow);

        tileset_ptr.draw(dstSurface, iTileSize, src_rect, dst_rect);
    }

    pub fn classic_tileset_index(&self) -> usize {
        self.m_classicTilesetIndex
    }

    /// Null when there is no "Classic" tileset.
    pub fn classic_tileset(&mut self) -> Ptr<CTileset> {
        self.tileset(self.classic_tileset_index())
    }

    pub fn tileset(&mut self, index: usize) -> Ptr<CTileset> {
        if index < self.m_tilesets.len() {
            self.m_tilesets[index].ensure_loaded();
            Ptr::from_mut(&mut *self.m_tilesets[index])
        } else {
            Ptr::null()
        }
    }

    /// `rect(DrawSize size, size_t col, size_t row)`
    pub fn rect(&mut self, size_id: i16, col: i16, row: i16) -> *mut SDL_Rect {
        let rect_idx = (row as isize as usize).wrapping_mul(MAX_TILES_PER_AXIS as usize).wrapping_add(col as isize as usize);
        self.rect_idx(size_id, rect_idx)
    }

    /// `rect(DrawSize size, size_t idx)`; out of range panics like `std::array::at`.
    pub fn rect_idx(&mut self, size_id: i16, idx: usize) -> *mut SDL_Rect {
        match size_id {
            1 => &mut self.m_rects_preview[idx],
            2 => &mut self.m_rects_thumb[idx],
            _ => &mut self.m_rects_ingame[idx],
        }
    }

    pub fn save_tilesets(&self) {
        for tileset in &self.m_tilesets {
            if tileset.is_loaded() && tileset.is_modified() {
                tileset.save_tileset();
            }
        }
    }
}
