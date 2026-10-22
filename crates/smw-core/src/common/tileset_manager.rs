//! Tilesets: the images and per-tile collision types maps draw from, loaded lazily from the graphics pack.

use crate::common::gfx::soft_surface::{BlendMode, Surface, SurfacePtr};
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::file_io::BinaryFile;
use crate::common::gfx::gfx_sprite::{gfxSprite, ImageLoader};
use crate::common::global_constants::*;
use crate::common::path::{convert_path, convert_path_pack};
use crate::common::tile_types::*;
use crate::common::util::container_helpers;
use crate::common::util::dir_iterator::SubdirsIterator;
use crate::globals::{Aliased, Ptr};
use std::collections::HashSet;
use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;

/// Largest tileset width or height in tiles the map format supports.
pub const MAX_TILES_PER_AXIS: i32 = 128;
pub const MAX_TILES: i32 = MAX_TILES_PER_AXIS * MAX_TILES_PER_AXIS;

/// Pixel rectangles of every tile in a 128x128 grid of `tilesize`-pixel tiles, row-major.
fn generate_tileset_rects(tilesize: i32) -> Vec<Rect> {
    let mut rects = Vec::with_capacity(MAX_TILES as usize);
    for row in 0..MAX_TILES_PER_AXIS {
        for col in 0..MAX_TILES_PER_AXIS {
            rects.push(Rect { x: col * tilesize, y: row * tilesize, w: tilesize, h: tilesize });
        }
    }
    rects
}

/// Reads a `tileset.tls` file: an `i32` count, then one `i32` tile type per tile, row-major. A missing or invalid
/// file gives no types, so every tile reads as non-solid.
fn read_tile_type_file(path: &Path) -> Vec<TileType> {
    let path_str = path.to_string_lossy().into_owned();
    println!("Reading {}", path_str);
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

/// Loads `path`, or on failure halves `largeRes` to stand in for it.
fn load_image_or_downscale(g: &mut Game, path: &Path, largeRes: &gfxSprite) -> gfxSprite {
    let mut gp = g.ptr();
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ImageLoader::new(path).create(gp.get()))) {
        Ok(sprite) => sprite,
        Err(err) => {
            let msg = err.downcast_ref::<String>().cloned().unwrap_or_default();
            println!("\nwarning: {} -> falling back to downscaled image", msg);

            unsafe {
                let large = largeRes.get_surface();
                let alpha = (*large).alpha;
                let blend = if alpha { BlendMode::Blend } else { BlendMode::None };
                let surf = SurfacePtr::new(Surface { alpha, blend, ..Surface::new(largeRes.get_width() / 2, largeRes.get_height() / 2) });
                crate::common::gfx::blit::upper_blit(large, null_mut(), surf.get(), null_mut());

                gfxSprite::from_surface(surf, None)
            }
        }
    }
}

/// The last component of `path`; empty for a path ending in `/`.
fn path_filename(path: &str) -> String {
    if path.ends_with('/') {
        return String::new();
    }
    Path::new(path).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default()
}

/*********************************
*  CTileset
*********************************/

/// One tileset directory: `large.png`, `medium.png` and `small.png` at the three draw sizes, and `tileset.tls`.
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

    /// Loads the images and tile types on first use; a missing medium or small image is downscaled from the larger one.
    pub fn ensure_loaded(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if self.is_loaded() {
            return;
        }

        self.m_tiletypes = read_tile_type_file(&self.m_tileset_dir.join("tileset.tls"));

        self.m_sprite_large = ImageLoader::new(self.m_tileset_dir.join("large.png")).create(gp.get());
        self.m_sprite_medium = load_image_or_downscale(g, &self.m_tileset_dir.join("medium.png"), &self.m_sprite_large);
        self.m_sprite_small = load_image_or_downscale(g, &self.m_tileset_dir.join("small.png"), &self.m_sprite_medium);

        self.m_width = (self.m_sprite_large.get_width() / TILESIZE).min(MAX_TILES_PER_AXIS) as i16;
        self.m_height = (self.m_sprite_large.get_height() / TILESIZE).min(MAX_TILES_PER_AXIS) as i16;
    }

    pub fn is_loaded(&self) -> bool {
        !self.m_sprite_large.get_surface().is_null()
    }

    /// Whether tile types were edited since loading.
    pub fn is_modified(&self) -> bool {
        self.m_modified
    }

    /// The tileset image at draw size `size`: 0 in-game (32-pixel tiles), 1 preview (16), 2 thumbnail (8).
    pub fn sprite(&self, size: i16) -> &gfxSprite {
        debug_assert!(self.is_loaded()); // call ensure_loaded first
        match size {
            1 => &self.m_sprite_medium,
            2 => &self.m_sprite_small,
            _ => &self.m_sprite_large,
        }
    }

    pub fn surface(&self, size: usize) -> *mut Surface {
        self.sprite(size as i16).get_surface()
    }

    /// Collision type of tile (`tileCol`, `tileRow`); tiles past the end of the type list are non-solid.
    pub fn tile_type(&self, tileCol: usize, tileRow: usize) -> TileType {
        let idx = tileCol.wrapping_add(tileRow.wrapping_mul(self.m_width as usize));
        self.m_tiletypes.get(idx).copied().unwrap_or(TileType::NonSolid)
    }

    /// Sets a tile's collision type, growing the type list as needed, and marks the tileset modified.
    pub fn set_tile_type(&mut self, tileCol: usize, tileRow: usize, r#type: TileType) {
        let idx = tileCol + tileRow * self.m_width as usize;
        if self.m_tiletypes.len() <= idx {
            self.m_tiletypes.resize(idx + 1, TileType::NonSolid);
        }
        self.m_tiletypes[idx] = r#type;
        self.m_modified = true;
    }

    /// Moves a tile to the next type in editor cycle order and returns it.
    pub fn increment_tile_type(&mut self, tileCol: usize, tileRow: usize) -> TileType {
        let new_type = next_tile_type(self.tile_type(tileCol, tileRow));
        self.set_tile_type(tileCol, tileRow, new_type);
        new_type
    }

    /// Moves a tile to the previous type in editor cycle order and returns it.
    pub fn decrement_tile_type(&mut self, tileCol: usize, tileRow: usize) -> TileType {
        let new_type = prev_tile_type(self.tile_type(tileCol, tileRow));
        self.set_tile_type(tileCol, tileRow, new_type);
        new_type
    }

    /// Blits `srcRect` of the image at draw size `tileSize` to `dstRect` of `dstSurface`.
    pub fn draw(&self, dstSurface: *mut Surface, tileSize: i16, srcRect: *mut Rect, dstRect: *mut Rect) {
        unsafe {
            crate::common::gfx::blit::upper_blit(self.surface(tileSize as usize), srcRect, dstSurface, dstRect);
        }
    }

    /// Writes the tile types back to `tileset.tls`.
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

        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tileset_path, std::fs::Permissions::from_mode(0o774));
        }
    }

    pub fn name(&self) -> &str {
        &self.m_name
    }
    /// Size in tiles, at most 128 per axis.
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

/// Every tileset in the current graphics pack, plus Classic-pack tilesets it lacks, sorted by name.
pub struct CTilesetManager {
    m_tilesets: Vec<Box<CTileset>>,
    m_classicTilesetIndex: usize,

    m_rects_ingame: Vec<Rect>,
    m_rects_preview: Vec<Rect>,
    m_rects_thumb: Vec<Rect>,

    pub _alias: Aliased,
}

impl CTilesetManager {
    pub fn new(g: &mut Game, gfxPack: &str) -> Self {
        let mut m_tilesets: Vec<Box<CTileset>> = Vec::new();
        let mut found_tileset_names: HashSet<String> = HashSet::new();

        let tilesets = convert_path_pack(g, "gfx/packs/tilesets", gfxPack) + "/";
        let mut dir = SubdirsIterator::new(g, tilesets);
        while let Some(path) = dir.next() {
            let tileset = Box::new(CTileset::new(path));
            found_tileset_names.insert(tileset.name().to_string());
            m_tilesets.push(tileset);
        }

        //Add tilesets from the Classic pack to fill the gaps
        if path_filename(gfxPack) != "Classic" {
            let classic = convert_path(g, "gfx/packs/Classic/tilesets/");
            dir = SubdirsIterator::new(g, classic);
            while let Some(path) = dir.next() {
                let name = path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                if !found_tileset_names.contains(&name) {
                    m_tilesets.push(Box::new(CTileset::new(path)));
                }
            }
        }

        container_helpers::sort(&mut m_tilesets, |a, b| a.name().as_bytes() < b.name().as_bytes());

        let m_classicTilesetIndex = m_tilesets.iter().position(|t| t.name() == "Classic").unwrap_or(usize::MAX);
        crate::common::gfx::blit::prefetch(
            m_tilesets.iter().flat_map(|t| ["large.png", "medium.png", "small.png"].map(|f| t.m_tileset_dir.join(f))).collect(),
        );

        CTilesetManager {
            m_tilesets,
            m_classicTilesetIndex,
            m_rects_ingame: generate_tileset_rects(TILESIZE),
            m_rects_preview: generate_tileset_rects(PREVIEWTILESIZE),
            m_rects_thumb: generate_tileset_rects(THUMBTILESIZE),
            _alias: Aliased::new(),
        }
    }

    /// Index of the tileset called `name`, or `TILESETUNKNOWN` cast to `usize` when there is none.
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

    /// Draws tile (`iSrcTileCol`, `iSrcTileRow`) of tileset `iTilesetID` at map cell (`iDstTileCol`, `iDstTileRow`), at
    /// draw size `iTileSize`; does nothing for an invalid tileset.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self, g: &mut Game, dstSurface: *mut Surface,
        iTilesetID: i16,
        iTileSize: i16,
        iSrcTileCol: i16,
        iSrcTileRow: i16,
        iDstTileCol: i16,
        iDstTileRow: i16,
    ) {
        let tileset_ptr = self.tileset(g, iTilesetID as isize as usize);
        if tileset_ptr.is_null() {
            return;
        }

        let src_rect = self.rect(iTileSize, iSrcTileCol, iSrcTileRow);
        let dst_rect = self.rect(iTileSize, iDstTileCol, iDstTileRow);

        tileset_ptr.draw(dstSurface, iTileSize, src_rect, dst_rect);
    }

    /// Index of the `Classic` tileset, or `usize::MAX` when there is none.
    pub fn classic_tileset_index(&self) -> usize {
        self.m_classicTilesetIndex
    }

    /// Null when there is no "Classic" tileset.
    pub fn classic_tileset(&mut self, g: &mut Game) -> Ptr<CTileset> {
        self.tileset(g, self.classic_tileset_index())
    }

    /// The tileset at `index`, loading it first; null when out of range.
    pub fn tileset(&mut self, g: &mut Game, index: usize) -> Ptr<CTileset> {
        if index < self.m_tilesets.len() {
            self.m_tilesets[index].ensure_loaded(g);
            Ptr::from_mut(&mut *self.m_tilesets[index])
        } else {
            Ptr::null()
        }
    }

    /// Pixel rectangle of tile (`col`, `row`) at draw size `size_id`; panics outside the 128x128 grid.
    pub fn rect(&mut self, size_id: i16, col: i16, row: i16) -> *mut Rect {
        let rect_idx = (row as isize as usize).wrapping_mul(MAX_TILES_PER_AXIS as usize).wrapping_add(col as isize as usize);
        self.rect_idx(size_id, rect_idx)
    }

    /// Pixel rectangle of tile `idx` (`row * 128 + col`) at draw size `size_id`; panics when out of range.
    pub fn rect_idx(&mut self, size_id: i16, idx: usize) -> *mut Rect {
        match size_id {
            1 => &mut self.m_rects_preview[idx],
            2 => &mut self.m_rects_thumb[idx],
            _ => &mut self.m_rects_ingame[idx],
        }
    }

    /// Saves the tile types of every edited tileset.
    pub fn save_tilesets(&self) {
        for tileset in &self.m_tilesets {
            if tileset.is_loaded() && tileset.is_modified() {
                tileset.save_tileset();
            }
        }
    }
}
