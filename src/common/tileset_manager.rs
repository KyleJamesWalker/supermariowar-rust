//! Port of src/common/TilesetManager.cpp

use crate::common::file_io::BinaryFile;
use crate::common::file_list::SimpleDirectoryList;
use crate::common::gfx::gfx_sprite::{gfxSprite, SpriteBuilder};
use crate::common::global_constants::*;
use crate::common::path::{convert_path, convert_path_pack, file_exists, get_filename_from_path};
use crate::common::tile_types::*;
use crate::common::util::container_helpers;
use crate::globals::{Aliased, Ptr};
use crate::impl_base;
use sdl2::sys::{SDL_Rect, SDL_Surface, SDL_UpperBlit};
use std::ptr::null_mut;

const MAX_TILES_PER_AXIS: i32 = 128;

fn read_tile_type_file(path: &str, expectedCount: i32) -> Vec<TileType> {
    const MAX_TILES: i32 = MAX_TILES_PER_AXIS * MAX_TILES_PER_AXIS;

    if !file_exists(path) {
        return Vec::new();
    }

    let mut tsf = BinaryFile::new(path, "rb");
    if !tsf.is_open() {
        println!("ERROR: couldn't open tileset file: {}", path);
        return Vec::new();
    }

    let mut tiletype_count = tsf.read_i32();
    if tiletype_count <= 0 || tiletype_count > MAX_TILES {
        return Vec::new();
    }

    tiletype_count = tiletype_count.min(expectedCount);

    let mut tiletypes = Vec::with_capacity(tiletype_count.max(0) as usize);
    for _ in 0..tiletype_count {
        tiletypes.push(TileType::from_i32(tsf.read_i32()));
    }

    tiletypes
}

pub struct CTileset {
    m_name: String,
    m_tilesetPath: String,

    m_sprites: [gfxSprite; 3],

    m_width: i16,
    m_height: i16,
    m_tiletypes: Vec<TileType>,
    pub _alias: Aliased,
}

impl CTileset {
    pub fn new(dir: &str) -> Self {
        let m_sprites = [
            SpriteBuilder::new(format!("{}/large.png", dir)).create(),
            SpriteBuilder::new(format!("{}/medium.png", dir)).create(),
            SpriteBuilder::new(format!("{}/small.png", dir)).create(),
        ];

        let m_width = (m_sprites[0].get_width() / TILESIZE).min(MAX_TILES_PER_AXIS) as i16;
        let m_height = (m_sprites[0].get_height() / TILESIZE).min(MAX_TILES_PER_AXIS) as i16;
        let m_tilesetPath = format!("{}/tileset.tls", dir);
        let m_tiletypes = read_tile_type_file(&m_tilesetPath, m_width as i32 * m_height as i32);

        CTileset { _alias: Aliased::new(), m_name: get_filename_from_path(dir), m_tilesetPath, m_sprites, m_width, m_height, m_tiletypes }
    }

    pub fn surface(&self, index: usize) -> *mut SDL_Surface {
        if index < self.m_sprites.len() {
            self.m_sprites[index].get_surface()
        } else {
            null_mut()
        }
    }

    /// Out of range reads heap garbage in C++ (reachable: see `classic_tileset`); the port returns `NonSolid`.
    pub fn tile_type(&self, tileCol: usize, tileRow: usize) -> TileType {
        let idx = tileCol.wrapping_add(tileRow.wrapping_mul(self.m_width as usize));
        self.m_tiletypes.get(idx).copied().unwrap_or(TileType::NonSolid)
    }

    pub fn set_tile_type(&mut self, tileCol: usize, tileRow: usize, r#type: TileType) {
        let w = self.m_width as usize;
        self.m_tiletypes[tileCol + tileRow * w] = r#type;
    }

    pub fn increment_tile_type(&mut self, tileCol: usize, tileRow: usize) -> TileType {
        let idx = tileCol + tileRow * self.m_width as usize;
        self.m_tiletypes[idx] = next_tile_type(self.m_tiletypes[idx]);
        self.m_tiletypes[idx]
    }

    pub fn decrement_tile_type(&mut self, tileCol: usize, tileRow: usize) -> TileType {
        let idx = tileCol + tileRow * self.m_width as usize;
        self.m_tiletypes[idx] = prev_tile_type(self.m_tiletypes[idx]);
        self.m_tiletypes[idx]
    }

    pub fn draw(&self, dstSurface: *mut SDL_Surface, tileSize: i16, srcRect: *mut SDL_Rect, dstRect: *mut SDL_Rect) {
        unsafe {
            SDL_UpperBlit(self.surface(tileSize as usize), srcRect, dstSurface, dstRect);
        }
    }

    pub fn save_tileset(&self) {
        let mut tsf = BinaryFile::new(&self.m_tilesetPath, "wb");
        if !tsf.is_open() {
            println!("ERROR: couldn't open tileset file to save tile types: {}", self.m_tilesetPath);
            return;
        }

        tsf.write_i32(self.m_tiletypes.len() as i32);

        for tiletype in &self.m_tiletypes {
            tsf.write_i32(tiletype.0 as i32);
        }
        drop(tsf);

        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&self.m_tilesetPath, std::fs::Permissions::from_mode(0o774));
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

pub struct CTilesetManager {
    pub simple_directory_list: SimpleDirectoryList,

    m_tilesetlist: Vec<Box<CTileset>>,
    m_classicTilesetIndex: usize,

    m_rects_ingame: Vec<SDL_Rect>,
    m_rects_preview: Vec<SDL_Rect>,
    m_rects_thumb: Vec<SDL_Rect>,
    m_max_tileset_cols: usize,

    pub _alias: Aliased,
}
impl_base!(CTilesetManager => simple_directory_list: SimpleDirectoryList);

impl CTilesetManager {
    pub fn new() -> Self {
        CTilesetManager {
            simple_directory_list: SimpleDirectoryList::new(convert_path("gfx/packs/Classic/tilesets/")),
            m_tilesetlist: Vec::new(),
            m_classicTilesetIndex: usize::MAX,
            m_rects_ingame: Vec::new(),
            m_rects_preview: Vec::new(),
            m_rects_thumb: Vec::new(),
            m_max_tileset_cols: 0,
            _alias: Aliased::new(),
        }
    }

    pub fn init(&mut self, gfxPack: &str) {
        self.m_tilesetlist.clear();

        if get_filename_from_path(gfxPack) != "Classic" {
            let s = convert_path_pack("gfx/packs/tilesets", gfxPack) + "/";
            let mut dirlist = SimpleDirectoryList::new(&s);

            for i in 0..dirlist.count() {
                let tileset = Box::new(CTileset::new(&dirlist.current_path().to_string_lossy()));

                if tileset.name() == "Classic" {
                    self.m_classicTilesetIndex = i;
                }

                self.m_tilesetlist.push(tileset);
                dirlist.next();
            }
        }

        for i in 0..self.m_filelist.len() {
            let tilesetName = get_filename_from_path(&self.m_filelist[i].to_string_lossy());

            let fFound = self.m_tilesetlist.iter().any(|t| t.name() == tilesetName);

            if !fFound {
                let tileset = Box::new(CTileset::new(&self.m_filelist[i].to_string_lossy()));
                if tileset.name() == "Classic" {
                    self.m_classicTilesetIndex = i;
                }
                self.m_tilesetlist.push(tileset);
            }
        }

        container_helpers::sort(&mut self.m_tilesetlist, |a, b| a.name().as_bytes() < b.name().as_bytes());

        self.init_tileset_rects();
    }

    fn init_tileset_rects(&mut self) {
        let mut max_tileset_rows: usize = 0;
        for tileset in &self.m_tilesetlist {
            max_tileset_rows = max_tileset_rows.max(tileset.height() as usize);
            self.m_max_tileset_cols = self.m_max_tileset_cols.max(tileset.width() as usize);
        }

        let tile_count = max_tileset_rows * self.m_max_tileset_cols;
        self.m_rects_ingame.reserve(tile_count);
        self.m_rects_preview.reserve(tile_count);
        self.m_rects_thumb.reserve(tile_count);

        let (mut y1, mut y2, mut y3): (i16, i16, i16) = (0, 0, 0);
        for _row in 0..max_tileset_rows {
            let (mut x1, mut x2, mut x3): (i16, i16, i16) = (0, 0, 0);
            for _col in 0..self.m_max_tileset_cols {
                self.m_rects_ingame.push(SDL_Rect { x: x1 as i32, y: y1 as i32, w: TILESIZE, h: TILESIZE });
                self.m_rects_preview.push(SDL_Rect { x: x2 as i32, y: y2 as i32, w: PREVIEWTILESIZE, h: PREVIEWTILESIZE });
                self.m_rects_thumb.push(SDL_Rect { x: x3 as i32, y: y3 as i32, w: THUMBTILESIZE, h: THUMBTILESIZE });

                x1 = (x1 as i32 + TILESIZE) as i16;
                x2 = (x2 as i32 + PREVIEWTILESIZE) as i16;
                x3 = (x3 as i32 + THUMBTILESIZE) as i16;
            }

            y1 = (y1 as i32 + TILESIZE) as i16;
            y2 = (y2 as i32 + PREVIEWTILESIZE) as i16;
            y3 = (y3 as i32 + THUMBTILESIZE) as i16;
        }
    }

    /// Returns `TILESETUNKNOWN` converted to `size_t` when not found, like C++.
    pub fn index_from_name(&self, name: &str) -> usize {
        for (i, t) in self.m_tilesetlist.iter().enumerate() {
            if t.name() == name {
                return i;
            }
        }

        TILESETUNKNOWN as isize as usize
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
        let src_rect_idx = (iSrcTileRow as isize as usize).wrapping_mul(self.m_max_tileset_cols).wrapping_add(iSrcTileCol as isize as usize);
        let dst_rect_idx = (iDstTileRow as isize as usize).wrapping_mul(self.m_max_tileset_cols).wrapping_add(iDstTileCol as isize as usize);

        let src_rect = self.rect_idx(iTileSize, src_rect_idx);
        let dst_rect = self.rect_idx(iTileSize, dst_rect_idx);

        self.m_tilesetlist[iTilesetID as usize].draw(dstSurface, iTileSize, src_rect, dst_rect);
    }

    pub fn classic_tileset_index(&self) -> usize {
        self.m_classicTilesetIndex
    }

    /// `m_classicTilesetIndex` is taken before `init` sorts the list, so this is usually not "Classic" (C++ bug).
    pub fn classic_tileset(&self) -> &CTileset {
        &self.m_tilesetlist[self.classic_tileset_index()]
    }

    pub fn tileset(&mut self, index: usize) -> Ptr<CTileset> {
        if index < self.m_tilesetlist.len() {
            Ptr::from_mut(&mut *self.m_tilesetlist[index])
        } else {
            Ptr::null()
        }
    }

    /// `rect(short size_id, short col, short row)`
    pub fn rect(&mut self, size_id: i16, col: i16, row: i16) -> *mut SDL_Rect {
        let rect_idx = (row as isize as usize).wrapping_mul(self.m_max_tileset_cols).wrapping_add(col as isize as usize);
        self.rect_idx(size_id, rect_idx)
    }

    /// `rect(short size_id, size_t idx)`
    pub fn rect_idx(&mut self, size_id: i16, idx: usize) -> *mut SDL_Rect {
        match size_id {
            0 => &mut self.m_rects_ingame[idx],
            1 => &mut self.m_rects_preview[idx],
            2 => &mut self.m_rects_thumb[idx],
            _ => null_mut(),
        }
    }

    pub fn save_tilesets(&self) {
        for tileset in &self.m_tilesetlist {
            tileset.save_tileset();
        }
    }
}
