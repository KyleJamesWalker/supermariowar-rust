//! Port of src/common/eyecandy.cpp
//!
//! Every class in the hierarchy below `EC_Animated` is a leaf, so `animate()` (virtual in C++)
//! is resolved statically: each `update()` calls the `animate` of its own class.

use crate::common::game::App;
use crate::common::gfx::gfx_font::gfxFont;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::object_base::cap_falling_velocity;
use crate::common::random_number_generator::{RANDOM_BOOL, RANDOM_INT};
use crate::common::tile_types::TileType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::eyecandy;
use sdl2::sys::{SDL_FillRect, SDL_Rect};
use std::ptr::null;

const screenWidth: i32 = App::screenWidth;
const screenHeight: i32 = App::screenHeight;

// eyecandy base class
pub trait CEyecandyTrait {
    fn update(&mut self);
    fn draw(&self);
    fn is_dead(&self) -> bool;
}

#[derive(Default)]
pub struct CEyecandy {
    pub dead: bool,
    pub _alias: Aliased,
}

impl CEyecandy {
    pub fn is_dead(&self) -> bool {
        self.dead
    }
}

macro_rules! impl_eyecandy {
    ($ty:ty) => {
        impl CEyecandyTrait for $ty {
            fn update(&mut self) {
                <$ty>::update(self)
            }
            fn draw(&self) {
                <$ty>::draw(self)
            }
            fn is_dead(&self) -> bool {
                self.dead
            }
        }
    };
}

#[inline]
fn spr(p: Ptr<gfxSprite>) -> &'static gfxSprite {
    p.get()
}

//------------------------------------------------------------------------------
// EC_StillImage
//------------------------------------------------------------------------------
pub struct EC_StillImage {
    pub c_eyecandy: CEyecandy,
    pub spr: Ptr<gfxSprite>,
    pub iSrcX: i16,
    pub iSrcY: i16,
    pub ix: i16,
    pub iy: i16,
    pub iw: i16,
    pub ih: i16,
}
impl_base!(EC_StillImage => c_eyecandy: CEyecandy);

impl EC_StillImage {
    pub fn new(nspr: Ptr<gfxSprite>, dstx: i16, dsty: i16, srcx: i16, srcy: i16, w: i16, h: i16) -> Self {
        let mut this = EC_StillImage {
            c_eyecandy: CEyecandy::default(),
            spr: nspr,
            iSrcX: srcx,
            iSrcY: srcy,
            ix: dstx,
            iy: dsty,
            iw: w,
            ih: h,
        };

        if this.iw == 0 {
            this.iw = this.spr.get_width() as i16;
        }

        if this.ih == 0 {
            this.ih = this.spr.get_height() as i16;
        }
        this
    }

    pub fn draw(&self) {
        if !self.dead {
            spr(self.spr).draw_part(self.ix as i32, self.iy as i32, self.iSrcX as i32, self.iSrcY as i32, self.iw as i32, self.ih as i32);
        }
    }
}

//------------------------------------------------------------------------------
// base class animated
//------------------------------------------------------------------------------
pub struct EC_Animated {
    pub c_eyecandy: CEyecandy,
    pub spr: Ptr<gfxSprite>,

    pub iAnimationX: i16,
    pub iAnimationY: i16,
    pub iAnimationW: i16,
    pub iAnimationH: i16,
    pub iAnimationSpeed: i16,
    pub iAnimationFrames: i16,

    pub iAnimationTimer: i16,
    pub iAnimationFrame: i16,

    pub ix: i16,
    pub iy: i16,
}
impl_base!(EC_Animated => c_eyecandy: CEyecandy);

impl EC_Animated {
    pub fn new(nspr: Ptr<gfxSprite>, dstx: i16, dsty: i16, srcx: i16, srcy: i16, w: i16, h: i16, speed: i16, frames: i16) -> Self {
        let mut this = EC_Animated {
            c_eyecandy: CEyecandy::default(),
            spr: nspr,
            iAnimationX: srcx,
            iAnimationY: srcy,
            iAnimationW: w,
            iAnimationH: h,
            iAnimationSpeed: 0,
            iAnimationFrames: 0,
            iAnimationTimer: 0,
            iAnimationFrame: 0,
            ix: 0,
            iy: 0,
        };

        if this.iAnimationW == 0 {
            this.iAnimationW = this.spr.get_width() as i16;
        }

        if this.iAnimationH == 0 {
            this.iAnimationH = this.spr.get_height() as i16;
        }

        this.iAnimationSpeed = speed;
        this.iAnimationFrames = (frames as i32 * this.iAnimationW as i32 + this.iAnimationX as i32) as i16;

        this.iAnimationTimer = 0;
        this.iAnimationFrame = this.iAnimationX;

        this.ix = dstx;
        this.iy = dsty;
        this
    }

    pub fn animate(&mut self) {
        if self.iAnimationSpeed > 0 && {
            self.iAnimationTimer += 1;
            self.iAnimationTimer >= self.iAnimationSpeed
        } {
            self.iAnimationTimer = 0;
            self.iAnimationFrame = (self.iAnimationFrame as i32 + self.iAnimationW as i32) as i16;

            if self.iAnimationFrame >= self.iAnimationFrames {
                self.iAnimationFrame = self.iAnimationX;
            }
        }
    }

    pub fn update(&mut self) {
        self.animate();
    }

    pub fn draw(&self) {
        if !self.dead {
            spr(self.spr).draw_part(
                self.ix as i32,
                self.iy as i32,
                self.iAnimationFrame as i32,
                self.iAnimationY as i32,
                self.iAnimationW as i32,
                self.iAnimationH as i32,
            );
        }
    }
}
impl_eyecandy!(EC_Animated);

//------------------------------------------------------------------------------
// base class OscillatingAnimation
//------------------------------------------------------------------------------
pub struct EC_OscillatingAnimation {
    pub ec_animated: EC_Animated,
    pub fForward: bool,
}
impl_base!(EC_OscillatingAnimation => ec_animated: EC_Animated);

impl EC_OscillatingAnimation {
    pub fn new(nspr: Ptr<gfxSprite>, dstx: i16, dsty: i16, srcx: i16, srcy: i16, w: i16, h: i16, speed: i16, frames: i16) -> Self {
        EC_OscillatingAnimation { ec_animated: EC_Animated::new(nspr, dstx, dsty, srcx, srcy, w, h, speed, frames), fForward: true }
    }

    pub fn animate(&mut self) {
        if self.iAnimationSpeed > 0 && {
            self.iAnimationTimer += 1;
            self.iAnimationTimer >= self.iAnimationSpeed
        } {
            self.iAnimationTimer = 0;

            if self.fForward {
                self.iAnimationFrame = (self.iAnimationFrame as i32 + self.iAnimationW as i32) as i16;

                if self.iAnimationFrame as i32 >= self.iAnimationFrames as i32 - self.iAnimationW as i32 {
                    self.fForward = false;
                }
            } else {
                self.iAnimationFrame = (self.iAnimationFrame as i32 - self.iAnimationW as i32) as i16;

                if self.iAnimationFrame <= 0 {
                    self.fForward = true;
                }
            }
        }
    }

    pub fn update(&mut self) {
        self.animate();
    }

    pub fn draw(&self) {
        self.ec_animated.draw();
    }
}
impl_eyecandy!(EC_OscillatingAnimation);

//------------------------------------------------------------------------------
// class cloud
//------------------------------------------------------------------------------
pub struct EC_Cloud {
    pub ec_still_image: EC_StillImage,
    dx: f32,
    dy: f32,
    velx: f32,
}
impl_base!(EC_Cloud => ec_still_image: EC_StillImage);

impl EC_Cloud {
    pub fn new(nspr: Ptr<gfxSprite>, nx: f32, ny: f32, nvelx: f32, srcx: i16, srcy: i16, w: i16, h: i16) -> Self {
        EC_Cloud { ec_still_image: EC_StillImage::new(nspr, nx as i16, ny as i16, srcx, srcy, w, h), dx: nx, dy: ny, velx: nvelx }
    }

    pub fn update(&mut self) {
        self.dx += self.velx;

        if self.dx > screenWidth as f32 {
            self.dx -= screenWidth as f32;
        } else if self.dx < 0.0f32 {
            self.dx += screenWidth as f32;
        }

        self.ix = self.dx as i16;
    }

    pub fn draw(&self) {
        self.ec_still_image.draw();
    }
}
impl_eyecandy!(EC_Cloud);

//------------------------------------------------------------------------------
// class ghost
//------------------------------------------------------------------------------
pub struct EC_Ghost {
    pub ec_animated: EC_Animated,
    dx: f32,
    dy: f32,
    velx: f32,
}
impl_base!(EC_Ghost => ec_animated: EC_Animated);

impl EC_Ghost {
    pub fn new(nspr: Ptr<gfxSprite>, nx: f32, ny: f32, nvelx: f32, ianimationspeed: i16, inumframes: i16, srcx: i16, srcy: i16, w: i16, h: i16) -> Self {
        EC_Ghost {
            ec_animated: EC_Animated::new(nspr, nx as i16, ny as i16, srcx, srcy, w, h, ianimationspeed, inumframes),
            dx: nx,
            dy: ny,
            velx: nvelx,
        }
    }

    pub fn update(&mut self) {
        self.ec_animated.animate();

        self.dx += self.velx;

        if self.dx >= screenWidth as f32 {
            self.dx -= screenWidth as f32;
        } else if self.dx < 0.0f32 {
            self.dx += screenWidth as f32;
        }

        self.ix = self.dx as i16;
        self.iy = self.dy as i16;
    }

    pub fn draw(&self) {
        self.ec_animated.draw();
    }
}
impl_eyecandy!(EC_Ghost);

//------------------------------------------------------------------------------
// class leaf
//------------------------------------------------------------------------------
pub struct EC_Leaf {
    pub ec_oscillating_animation: EC_OscillatingAnimation,
    dx: f32,
    dy: f32,
    velx: f32,
    vely: f32,
}
impl_base!(EC_Leaf => ec_oscillating_animation: EC_OscillatingAnimation);

impl EC_Leaf {
    pub fn new(nspr: Ptr<gfxSprite>, nx: f32, ny: f32) -> Self {
        let mut this = EC_Leaf {
            ec_oscillating_animation: EC_OscillatingAnimation::new(nspr, nx as i16, ny as i16, 0, 0, 16, 16, 16, 4),
            dx: nx,
            dy: ny,
            velx: 0.0,
            vely: 0.0,
        };

        // Create random drag for each leaf to give the effect some variance
        this.next_leaf();
        this
    }

    pub fn update(&mut self) {
        self.ec_oscillating_animation.animate();

        unsafe {
            self.dx += self.velx + game_values.flags.gamewindx;
            self.dy += self.vely + game_values.flags.gamewindy;
        }

        if self.dx >= screenWidth as f32 {
            self.dx -= screenWidth as f32;
        } else if self.dx < 0.0f32 {
            self.dx += screenWidth as f32;
        }

        if self.vely > 0.0f32 && self.iy as i32 >= screenHeight {
            self.dy = -16.0f32;
            self.dx = RANDOM_INT(screenWidth) as f32;

            self.next_leaf();
        } else if self.vely < 0.0f32 && self.iy < -16 {
            self.dy = screenHeight as f32;
            self.dx = RANDOM_INT(screenWidth) as f32;

            self.next_leaf();
        }

        self.ix = self.dx as i16;
        self.iy = self.dy as i16;
    }

    fn next_leaf(&mut self) {
        let iRand = RANDOM_INT(20) as i16;
        if iRand < 12 {
            self.iAnimationY = 0;
        } else if iRand < 15 {
            self.iAnimationY = 16;
        } else if iRand < 18 {
            self.iAnimationY = 32;
        } else {
            self.iAnimationY = 48;
        }

        self.velx = RANDOM_INT(9) as f32 / 4.0f32;
        self.vely = RANDOM_INT(9) as f32 / 4.0f32 + 1.0f32;

        self.fForward = RANDOM_BOOL();
        self.iAnimationFrame = ((RANDOM_INT(3) + if self.fForward { 0 } else { 1 }) * self.iAnimationW as i32) as i16;
        self.iAnimationTimer = RANDOM_INT(16) as i16;
    }

    pub fn draw(&self) {
        self.ec_oscillating_animation.draw();
    }
}
impl_eyecandy!(EC_Leaf);

//------------------------------------------------------------------------------
// class snow
//------------------------------------------------------------------------------
pub struct EC_Snow {
    pub ec_still_image: EC_StillImage,
    dx: f32,
    dy: f32,
    velx: f32,
    vely: f32,
}
impl_base!(EC_Snow => ec_still_image: EC_StillImage);

impl EC_Snow {
    pub fn new(nspr: Ptr<gfxSprite>, nx: f32, ny: f32, r#type: i16) -> Self {
        let srcx = ((RANDOM_BOOL() as i32) << 4) as i16;
        let mut this = EC_Snow {
            ec_still_image: EC_StillImage::new(nspr, nx as i16, ny as i16, srcx, ((r#type as i32) << 4) as i16, 16, 16),
            dx: nx,
            dy: ny,
            velx: 0.0,
            vely: 0.0,
        };

        this.velx = RANDOM_INT(9) as f32 / 4.0f32;
        this.vely = RANDOM_INT(9) as f32 / 4.0f32 + 1.0f32;
        this
    }

    pub fn update(&mut self) {
        unsafe {
            self.dx += self.velx + game_values.flags.gamewindx;
            self.dy += self.vely + game_values.flags.gamewindy;
        }

        if self.dx >= screenWidth as f32 {
            self.dx -= screenWidth as f32;
        } else if self.dx < 0.0f32 {
            self.dx += screenWidth as f32;
        }

        if self.vely > 0.0f32 && self.iy as i32 >= screenHeight {
            self.dy = -16.0f32;
            self.dx = RANDOM_INT(screenWidth) as f32;
        } else if self.vely < 0.0f32 && self.iy < -16 {
            self.dy = screenHeight as f32;
            self.dx = RANDOM_INT(screenWidth) as f32;
        }

        self.ix = self.dx as i16;
        self.iy = self.dy as i16;
    }

    pub fn draw(&self) {
        self.ec_still_image.draw();
    }
}
impl_eyecandy!(EC_Snow);

//------------------------------------------------------------------------------
// class rain
//------------------------------------------------------------------------------
pub struct EC_Rain {
    pub ec_still_image: EC_StillImage,
    dx: f32,
    dy: f32,
    velx: f32,
    vely: f32,
}
impl_base!(EC_Rain => ec_still_image: EC_StillImage);

impl EC_Rain {
    pub fn new(nspr: Ptr<gfxSprite>, nx: f32, ny: f32) -> Self {
        let mut this = EC_Rain {
            ec_still_image: EC_StillImage::new(nspr, nx as i16, ny as i16, 0, 16, 10, 10),
            dx: 0.0,
            dy: 0.0,
            velx: 0.0,
            vely: 0.0,
        };
        this.next_rain_drop();
        this.dx = nx;
        this.dy = ny;
        this
    }

    pub fn update(&mut self) {
        self.dx += self.velx;
        self.dy += self.vely;

        // If rain is off left edge, wrap it
        if self.dx < 0.0f32 {
            self.dx += screenWidth as f32;
        }

        // If rain is off bottom edge, change the rain gfx and start it from the top
        if self.iy as i32 >= screenHeight {
            self.next_rain_drop();
        }

        self.ix = self.dx as i16;
        self.iy = self.dy as i16;
    }

    fn next_rain_drop(&mut self) {
        self.velx = -5.0f32 + RANDOM_INT(5) as f32 / 4.0f32;
        self.vely = 4.0f32 + RANDOM_INT(5) as f32 / 4.0f32;

        self.dy = -16.0f32;
        self.dx = RANDOM_INT(screenWidth) as f32;

        self.iSrcX = (RANDOM_INT(8) * 10) as i16;
    }

    pub fn draw(&self) {
        self.ec_still_image.draw();
    }
}
impl_eyecandy!(EC_Rain);

//------------------------------------------------------------------------------
// class bubble
//------------------------------------------------------------------------------
pub struct EC_Bubble {
    pub ec_oscillating_animation: EC_OscillatingAnimation,
    dx: f32,
    dy: f32,
    velx: f32,
    vely: f32,
}
impl_base!(EC_Bubble => ec_oscillating_animation: EC_OscillatingAnimation);

impl EC_Bubble {
    pub fn new(nspr: Ptr<gfxSprite>, nx: f32, ny: f32) -> Self {
        let mut this = EC_Bubble {
            ec_oscillating_animation: EC_OscillatingAnimation::new(nspr, nx as i16, ny as i16, 0, 0, 16, 8, 4, 4),
            dx: 0.0,
            dy: 0.0,
            velx: 0.0,
            vely: 0.0,
        };
        this.next_bubble();
        this.dx = nx;
        this.dy = ny;
        this
    }

    pub fn update(&mut self) {
        self.ec_oscillating_animation.animate();

        self.dx += self.velx;
        self.dy += self.vely;

        // If bubble is off the edges, wrap it
        if self.dx < 0.0f32 {
            self.dx += screenWidth as f32;
        } else if self.dx + self.iAnimationW as f32 >= screenWidth as f32 {
            self.dx -= screenWidth as f32;
        }

        // If bubble is off top edge, move it back to the bottom to start again
        if (self.iy as i32 + self.iAnimationH as i32) < 0 {
            self.next_bubble();
        }

        self.ix = self.dx as i16;
        self.iy = self.dy as i16;
    }

    fn next_bubble(&mut self) {
        self.velx = -1.0f32 + RANDOM_INT(9) as f32 / 4.0f32;
        self.vely = -4.0f32 + RANDOM_INT(9) as f32 / 4.0f32;

        self.dy = screenHeight as f32;
        self.dx = RANDOM_INT(screenWidth) as f32;

        self.iAnimationFrame = (RANDOM_INT(4) << 4) as i16;
    }

    pub fn draw(&self) {
        self.ec_oscillating_animation.draw();
    }
}
impl_eyecandy!(EC_Bubble);

//------------------------------------------------------------------------------
// class corpse
//------------------------------------------------------------------------------
pub struct EC_Corpse {
    pub ec_still_image: EC_StillImage,
    dx: f32,
    dy: f32,
    tx: i16,
    tx2: i16,
    vely: f32,
    timeleft: i16,
    offsetx: i16,
}
impl_base!(EC_Corpse => ec_still_image: EC_StillImage);

impl EC_Corpse {
    pub fn new(nspr: Ptr<gfxSprite>, nx: f32, ny: f32, iSrcOffsetX: i16) -> Self {
        let mut this = EC_Corpse {
            ec_still_image: EC_StillImage::new(nspr, nx as i16, ny as i16, iSrcOffsetX, 0, 32, 32),
            dx: nx,
            dy: ny,
            tx: 0,
            tx2: 0,
            vely: GRAVITATION,
            timeleft: CORPSESTAY as i16,
            offsetx: iSrcOffsetX,
        };

        let ix = this.ix as i32;
        if ix + PWOFFSET < 0 {
            this.tx = ((ix + screenWidth + PWOFFSET) / TILESIZE) as i16;
        } else {
            this.tx = ((ix + PWOFFSET) / TILESIZE) as i16;
        }

        if ix + PWOFFSET + PW >= screenWidth {
            this.tx2 = ((ix + PWOFFSET + PW - screenWidth) / TILESIZE) as i16;
        } else {
            this.tx2 = ((ix + PWOFFSET + PW) / TILESIZE) as i16;
        }

        // Wrap around screen properly
        let width = (screenWidth / TILESIZE) as i16;
        if this.tx < 0 {
            this.tx += width;
        } else if this.tx >= width {
            this.tx -= width;
        }
        if this.tx2 < 0 {
            this.tx2 += width;
        } else if this.tx2 >= width {
            this.tx2 -= width;
        }
        this
    }

    pub fn update(&mut self) {
        unsafe {
            if self.vely != 0.0f32 {
                let nexty = (self.dy + 32.0f32 + self.vely) as i16;

                if nexty as i32 >= screenHeight {
                    self.dead = true;
                    return;
                }

                if nexty >= 0 {
                    let ty = (nexty as i32 / TILESIZE) as i16;

                    // FIXME: This seems to be wrong
                    if g_map.map(self.tx as i32, ty as i32) == TileType::SolidOnTop.0 as i32
                        || g_map.map(self.tx2 as i32, ty as i32) == TileType::SolidOnTop.0 as i32
                    {
                        // on ground on tile solid_on_top
                        if (self.dy + 32.0f32 - self.vely) / (TILESIZE as f32) < ty as f32 {
                            // only if we were above the tile in the previous frame
                            self.dy = (((ty as i32) << 5) - TILESIZE) as f32;
                            self.vely = 0.0f32;
                            self.iy = self.dy as i16;

                            return;
                        }
                    }

                    let mut leftblock = g_map.block(self.tx, ty);
                    let mut rightblock = g_map.block(self.tx2, ty);

                    if (g_map.map(self.tx as i32, ty as i32) & 0x13) > 0
                        || (g_map.map(self.tx2 as i32, ty as i32) & 0x13) > 0
                        || (!leftblock.is_null() && !leftblock.is_transparent() && !leftblock.is_hidden())
                        || (!rightblock.is_null() && !rightblock.is_transparent() && !rightblock.is_hidden())
                    {
                        // on ground
                        self.dy = (((ty as i32) << 5) - TILESIZE) as f32;
                        self.vely = 0.0f32;
                        self.iy = self.dy as i16;

                        return;
                    }
                }

                // falling (in air)
                self.dy += self.vely;
                self.vely = cap_falling_velocity(GRAVITATION + self.vely);
            } else {
                if self.timeleft > 0 {
                    self.timeleft -= 1;
                } else {
                    self.dead = true;
                }
            }

            self.iy = self.dy as i16;
        }
    }

    pub fn draw(&self) {
        self.ec_still_image.draw();
    }
}
impl_eyecandy!(EC_Corpse);

//------------------------------------------------------------------------------
// class EC_GravText
//------------------------------------------------------------------------------
pub struct EC_GravText {
    pub c_eyecandy: CEyecandy,
    font: Ptr<gfxFont>,
    x: f32,
    y: f32,
    w: i16,
    vely: f32,
    text: String,
}
impl_base!(EC_GravText => c_eyecandy: CEyecandy);

impl EC_GravText {
    pub fn new(nfont: Ptr<gfxFont>, nx: i16, ny: i16, ntext: String, nvely: f32) -> Self {
        let mut this = EC_GravText { c_eyecandy: CEyecandy::default(), font: nfont, x: 0.0, y: 0.0, w: 0, vely: nvely, text: ntext };
        this.x = (nx as i32 - (this.font.get_width(&this.text) / 2)) as f32;
        this.y = ny as f32;
        this.w = this.font.get_width(&this.text) as i16;
        this
    }

    pub fn update(&mut self) {
        self.y += self.vely;
        self.vely += GRAVITATION;

        if self.y > screenHeight as f32 {
            self.dead = true;
        }
    }

    pub fn draw(&self) {
        self.font.draw(self.x as i16 as i32, self.y as i16 as i32, &self.text);

        if self.x < 0.0 {
            self.font.draw(self.x as i16 as i32 + screenWidth, self.y as i16 as i32, &self.text);
        } else if self.x + self.w as f32 > (screenWidth - 1) as f32 {
            self.font.draw(self.x as i16 as i32 - screenWidth, self.y as i16 as i32, &self.text);
        }
    }
}
impl_eyecandy!(EC_GravText);

//------------------------------------------------------------------------------
// class EC_Announcement
//------------------------------------------------------------------------------
pub struct EC_Announcement {
    pub c_eyecandy: CEyecandy,
    font: Ptr<gfxFont>,
    sprite: Ptr<gfxSprite>,

    ix: i16,
    iy: i16,
    text: String,

    iTime: i16,
    iTimer: i16,
    iIcon: i16,

    iFontY: i16,
    iFontOffsetX: i16,

    rSrcRect: [SDL_Rect; 4],
    rDstRect: [SDL_Rect; 4],
}
impl_base!(EC_Announcement => c_eyecandy: CEyecandy);

impl EC_Announcement {
    pub fn new(nfont: Ptr<gfxFont>, nsprite: Ptr<gfxSprite>, ntext: String, icon: i16, time: i16, y: i16) -> Self {
        let zero = SDL_Rect { x: 0, y: 0, w: 0, h: 0 };
        let mut this = EC_Announcement {
            c_eyecandy: CEyecandy::default(),
            font: nfont,
            sprite: nsprite,
            ix: 0,
            iy: y,
            text: ntext,
            iTime: time,
            iTimer: 0,
            iIcon: icon,
            iFontY: 0,
            iFontOffsetX: 0,
            rSrcRect: [zero; 4],
            rDstRect: [zero; 4],
        };

        this.iFontY = (this.iy as i32 + 32 - (this.font.get_height() >> 1)) as i16;

        this.iFontOffsetX = (16 + if !this.sprite.is_null() { 48 } else { 0 }) as i16;

        let iWidth = (16 + this.iFontOffsetX as i32 + this.font.get_width(&this.text)) as i16;
        let iHalfWidth = iWidth >> 1;

        this.ix = (screenWidth / 2 - iHalfWidth as i32) as i16;

        let (ix, iy, hw) = (this.ix as i32, this.iy as i32, iHalfWidth as i32);
        this.rDstRect[0] = SDL_Rect { x: ix, y: iy, w: hw, h: 32 };
        this.rDstRect[1] = SDL_Rect { x: ix + hw, y: iy, w: hw, h: 32 };
        this.rDstRect[2] = SDL_Rect { x: ix, y: iy + 32, w: hw, h: 32 };
        this.rDstRect[3] = SDL_Rect { x: ix + hw, y: iy + 32, w: hw, h: 32 };

        this.rSrcRect[0] = SDL_Rect { x: 0, y: 0, w: hw, h: 32 };
        this.rSrcRect[1] = SDL_Rect { x: (screenWidth as f32 * 0.8f32 - hw as f32) as i32, y: 0, w: hw, h: 32 };
        this.rSrcRect[2] = SDL_Rect { x: 0, y: (screenHeight as f32 * 0.93f32) as i32, w: hw, h: 32 };
        this.rSrcRect[3] = SDL_Rect {
            x: (screenWidth as f32 * 0.8f32 - hw as f32) as i32,
            y: (screenHeight as f32 * 0.93f32) as i32,
            w: hw,
            h: 32,
        };
        this
    }

    pub fn update(&mut self) {
        self.iTimer += 1;
        if self.iTimer >= self.iTime {
            self.dead = true;
        }
    }

    pub fn draw(&self) {
        unsafe {
            for iRect in 0..4 {
                rm.menu_dialog.draw_part(
                    self.rDstRect[iRect].x,
                    self.rDstRect[iRect].y,
                    self.rSrcRect[iRect].x,
                    self.rSrcRect[iRect].y,
                    self.rSrcRect[iRect].w,
                    self.rSrcRect[iRect].h,
                );
            }
        }

        self.font.draw(self.ix as i32 + self.iFontOffsetX as i32, self.iFontY as i32, &self.text);

        if !self.sprite.is_null() {
            self.sprite.draw_part(self.ix as i32 + 16, self.iy as i32 + 16, (self.iIcon as i32) << 5, 0, 32, 32);
        }
    }
}
impl_eyecandy!(EC_Announcement);

//------------------------------------------------------------------------------
// class EC_FallingObject
//------------------------------------------------------------------------------
pub struct EC_FallingObject {
    pub ec_animated: EC_Animated,
    fx: f32,
    fy: f32,
    vely: f32,
    velx: f32,
}
impl_base!(EC_FallingObject => ec_animated: EC_Animated);

impl EC_FallingObject {
    pub fn new(
        nspr: Ptr<gfxSprite>,
        x: i16,
        y: i16,
        nvelx: f32,
        nvely: f32,
        animationframes: i16,
        animationspeed: i16,
        srcOffsetX: i16,
        srcOffsetY: i16,
        w: i16,
        h: i16,
    ) -> Self {
        EC_FallingObject {
            ec_animated: EC_Animated::new(nspr, x, y, srcOffsetX, srcOffsetY, w, h, animationspeed, animationframes),
            fx: x as f32,
            fy: y as f32,
            velx: nvelx,
            vely: nvely,
        }
    }

    pub fn update(&mut self) {
        self.ec_animated.animate();

        self.fy += self.vely;
        self.fx += self.velx;

        self.ix = self.fx as i16;
        self.iy = self.fy as i16;

        if self.fy >= screenHeight as f32 {
            self.dead = true;
            return;
        }

        self.vely += GRAVITATION;
    }

    pub fn draw(&self) {
        self.ec_animated.draw();
    }
}
impl_eyecandy!(EC_FallingObject);

//------------------------------------------------------------------------------
// class EC_SingleAnimation
//------------------------------------------------------------------------------
pub struct EC_SingleAnimation {
    pub ec_animated: EC_Animated,
}
impl_base!(EC_SingleAnimation => ec_animated: EC_Animated);

impl EC_SingleAnimation {
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, iframes: i16, irate: i16) -> Self {
        let w = (nspr.get_width() as i16 as i32 / iframes as i32) as i16;
        EC_SingleAnimation { ec_animated: EC_Animated::new(nspr, x, y, 0, 0, w, 0, irate, iframes) }
    }

    pub fn new_rect(nspr: Ptr<gfxSprite>, x: i16, y: i16, iframes: i16, irate: i16, offsetx: i16, offsety: i16, w: i16, h: i16) -> Self {
        EC_SingleAnimation { ec_animated: EC_Animated::new(nspr, x, y, offsetx, offsety, w, h, irate, iframes) }
    }

    pub fn update(&mut self) {
        self.ec_animated.animate();

        // This means that the animation wrapped and it is time to kill it
        if self.iAnimationTimer == 0 && self.iAnimationFrame == self.iAnimationX {
            self.dead = true;
        }
    }

    pub fn draw(&self) {
        self.ec_animated.draw();
    }
}
impl_eyecandy!(EC_SingleAnimation);

//------------------------------------------------------------------------------
// class EC_LoopingAnimation
//------------------------------------------------------------------------------
pub struct EC_LoopingAnimation {
    pub ec_animated: EC_Animated,
    iCountLoops: i16,
    iLoops: i16,
}
impl_base!(EC_LoopingAnimation => ec_animated: EC_Animated);

impl EC_LoopingAnimation {
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, iframes: i16, irate: i16, loops: i16, ioffsetx: i16, ioffsety: i16, w: i16, h: i16) -> Self {
        EC_LoopingAnimation {
            ec_animated: EC_Animated::new(nspr, x, y, ioffsetx, ioffsety, w, h, irate, iframes),
            iCountLoops: 0,
            iLoops: loops,
        }
    }

    pub fn update(&mut self) {
        self.ec_animated.animate();

        if self.iAnimationTimer == 0 && self.iAnimationFrame == self.iAnimationX {
            if self.iLoops > 0 && {
                self.iCountLoops += 1;
                self.iCountLoops >= self.iLoops
            } {
                self.dead = true;
            }
        }
    }

    pub fn draw(&self) {
        self.ec_animated.draw();
    }
}
impl_eyecandy!(EC_LoopingAnimation);

/// `eyecandy[2].emplace<EC_SingleAnimation>(&rm->spr_fireballexplosion, x, y, 3, 8)`
unsafe fn emplace_fireball_explosion(x: i16, y: i16) {
    let spr = Ptr::from_mut(&mut rm.spr_fireballexplosion);
    eyecandy[2].emplace(EC_SingleAnimation::new(spr, x, y, 3, 8));
}

//------------------------------------------------------------------------------
// class EC_ExplodingAward
//------------------------------------------------------------------------------
pub struct EC_ExplodingAward {
    pub c_eyecandy: CEyecandy,
    spr: Ptr<gfxSprite>,
    x: f32,
    y: f32,
    w: i16,
    h: i16,
    timer: i16,
    ttl: i16,
    id: i16,
    velx: f32,
    vely: f32,
}
impl_base!(EC_ExplodingAward => c_eyecandy: CEyecandy);

impl EC_ExplodingAward {
    pub fn new(nspr: Ptr<gfxSprite>, nx: i16, ny: i16, nvelx: f32, nvely: f32, timetolive: i16, awardID: i16) -> Self {
        let w: i16 = 16;
        EC_ExplodingAward {
            c_eyecandy: CEyecandy::default(),
            spr: nspr,
            x: nx as f32,
            y: ny as f32,
            vely: nvely,
            velx: nvelx,
            timer: 0,
            ttl: timetolive,
            w,
            h: 16,
            id: (awardID as i32 * w as i32) as i16,
        }
    }

    pub fn update(&mut self) {
        self.y += self.vely;
        self.x += self.velx;

        self.timer += 1;
        if self.timer > self.ttl {
            self.dead = true;
            unsafe {
                emplace_fireball_explosion(
                    (self.x as i16 as i32 + (self.w as i32 >> 1) - 16) as i16,
                    (self.y as i16 as i32 + (self.h as i32 >> 1) - 16) as i16,
                )
            };
        }
    }

    pub fn draw(&self) {
        spr(self.spr).draw_part(self.x as i16 as i32, self.y as i16 as i32, self.id as i32, 0, self.w as i32, self.h as i32);
    }
}
impl_eyecandy!(EC_ExplodingAward);

//------------------------------------------------------------------------------
// class EC_SwirlingAward
//------------------------------------------------------------------------------
pub struct EC_SwirlingAward {
    pub c_eyecandy: CEyecandy,
    spr: Ptr<gfxSprite>,
    x: i16,
    y: i16,
    w: i16,
    h: i16,
    timer: i16,
    ttl: i16,
    iSrcX: i16,
    iSrcY: i16,
    vel: f32,
    angle: f32,
    radius: f32,

    iAnimationRate: i16,
    iAnimationFrames: i16,
    iAnimationTimer: i16,
    iAnimationFrame: i16,
    iAnimationEndFrame: i16,
}
impl_base!(EC_SwirlingAward => c_eyecandy: CEyecandy);

impl EC_SwirlingAward {
    pub fn new(
        nspr: Ptr<gfxSprite>,
        nx: i16,
        ny: i16,
        nangle: f32,
        nradius: f32,
        nvel: f32,
        timetolive: i16,
        srcX: i16,
        srcY: i16,
        iw: i16,
        ih: i16,
        animationRate: i16,
        animationFrames: i16,
    ) -> Self {
        let iSrcX = (srcX as i32 * iw as i32) as i16;
        EC_SwirlingAward {
            c_eyecandy: CEyecandy::default(),
            spr: nspr,
            x: nx,
            y: ny,
            vel: nvel,
            angle: nangle,
            radius: nradius,
            timer: 0,
            ttl: timetolive,
            w: iw,
            h: ih,
            iSrcX,
            iSrcY: (srcY as i32 * ih as i32) as i16,
            iAnimationRate: animationRate,
            iAnimationFrames: animationFrames,
            iAnimationTimer: 0,
            iAnimationFrame: iSrcX,
            iAnimationEndFrame: (iSrcX as i32 + iw as i32 * animationFrames as i32) as i16,
        }
    }

    pub fn update(&mut self) {
        self.angle += self.vel;
        self.radius += 3.0f32;

        self.timer += 1;
        if self.timer > self.ttl {
            let awardx = (self.x as i32 + (self.radius * self.angle.cos()) as i16 as i32 + (self.w as i32 >> 1) - 16) as i16;
            let awardy = (self.y as i32 + (self.radius * self.angle.sin()) as i16 as i32 + (self.h as i32 >> 1) - 16) as i16;
            unsafe { emplace_fireball_explosion(awardx, awardy) };

            self.dead = true;
        }

        if self.iAnimationRate > 0 && {
            self.iAnimationTimer += 1;
            self.iAnimationTimer > self.iAnimationRate
        } {
            self.iAnimationTimer = 0;

            self.iAnimationFrame = (self.iAnimationFrame as i32 + self.w as i32) as i16;
            if self.iAnimationFrame >= self.iAnimationEndFrame {
                self.iAnimationFrame = self.iSrcX;
            }
        }
    }

    pub fn draw(&self) {
        let awardx = (self.x as i32 + (self.radius * self.angle.cos()) as i16 as i32) as i16;
        let awardy = (self.y as i32 + (self.radius * self.angle.sin()) as i16 as i32) as i16;

        spr(self.spr).draw_part(awardx as i32, awardy as i32, self.iAnimationFrame as i32, self.iSrcY as i32, self.w as i32, self.h as i32);
    }
}
impl_eyecandy!(EC_SwirlingAward);

//------------------------------------------------------------------------------
// class EC_RocketAward
//------------------------------------------------------------------------------
pub struct EC_RocketAward {
    pub c_eyecandy: CEyecandy,
    spr: Ptr<gfxSprite>,
    x: f32,
    y: f32,
    w: i16,
    h: i16,
    velx: f32,
    vely: f32,
    timer: i16,
    ttl: i16,
    iSrcX: i16,
    iSrcY: i16,

    iAnimationRate: i16,
    iAnimationFrames: i16,
    iAnimationTimer: i16,
    iAnimationFrame: i16,
    iAnimationEndFrame: i16,
}
impl_base!(EC_RocketAward => c_eyecandy: CEyecandy);

impl EC_RocketAward {
    pub fn new(
        nspr: Ptr<gfxSprite>,
        nx: i16,
        ny: i16,
        nvelx: f32,
        nvely: f32,
        timetolive: i16,
        srcX: i16,
        srcY: i16,
        iw: i16,
        ih: i16,
        animationRate: i16,
        animationFrames: i16,
    ) -> Self {
        let iSrcX = (srcX as i32 * iw as i32) as i16;
        EC_RocketAward {
            c_eyecandy: CEyecandy::default(),
            spr: nspr,
            x: nx as f32,
            y: ny as f32,
            velx: nvelx,
            vely: nvely,
            timer: 0,
            ttl: timetolive,
            w: iw,
            h: ih,
            iSrcX,
            iSrcY: (srcY as i32 * ih as i32) as i16,
            iAnimationRate: animationRate,
            iAnimationFrames: animationFrames,
            iAnimationTimer: 0,
            iAnimationFrame: iSrcX,
            iAnimationEndFrame: (iSrcX as i32 + iw as i32 * animationFrames as i32) as i16,
        }
    }

    pub fn update(&mut self) {
        self.vely += 0.2f32;

        self.y += self.vely;
        self.x += self.velx;

        self.timer += 1;
        if self.timer > self.ttl {
            unsafe {
                emplace_fireball_explosion(
                    (self.x as i16 as i32 + (self.w as i32 >> 1) - 16) as i16,
                    (self.y as i16 as i32 + (self.h as i32 >> 1) - 16) as i16,
                )
            };
            self.dead = true;
        }

        if self.iAnimationRate > 0 && {
            self.iAnimationTimer += 1;
            self.iAnimationTimer > self.iAnimationRate
        } {
            self.iAnimationTimer = 0;

            self.iAnimationFrame = (self.iAnimationFrame as i32 + self.w as i32) as i16;
            if self.iAnimationFrame >= self.iAnimationEndFrame {
                self.iAnimationFrame = self.iSrcX;
            }
        }
    }

    pub fn draw(&self) {
        spr(self.spr).draw_part(
            self.x as i16 as i32,
            self.y as i16 as i32,
            self.iAnimationFrame as i32,
            self.iSrcY as i32,
            self.w as i32,
            self.h as i32,
        );
    }
}
impl_eyecandy!(EC_RocketAward);

//------------------------------------------------------------------------------
// class EC_FloatingAward
//------------------------------------------------------------------------------
pub struct EC_FloatingObject {
    pub c_eyecandy: CEyecandy,
    spr: Ptr<gfxSprite>,
    x: f32,
    y: f32,
    w: i16,
    h: i16,
    velx: f32,
    vely: f32,
    timer: i16,
    ttl: i16,
    srcx: i16,
    srcy: i16,
}
impl_base!(EC_FloatingObject => c_eyecandy: CEyecandy);

impl EC_FloatingObject {
    pub fn new(
        nspr: Ptr<gfxSprite>,
        nx: i16,
        ny: i16,
        nvelx: f32,
        nvely: f32,
        timetolive: i16,
        nsrcx: i16,
        nsrcy: i16,
        nwidth: i16,
        nheight: i16,
    ) -> Self {
        EC_FloatingObject {
            c_eyecandy: CEyecandy::default(),
            spr: nspr,
            x: nx as f32,
            y: ny as f32,
            velx: nvelx,
            vely: nvely,
            srcx: nsrcx,
            srcy: nsrcy,
            w: nwidth,
            h: nheight,
            timer: 0,
            ttl: timetolive,
        }
    }

    pub fn update(&mut self) {
        self.y += self.vely;
        self.x += self.velx;

        self.timer += 1;
        if self.timer > self.ttl {
            unsafe {
                emplace_fireball_explosion(
                    (self.x as i16 as i32 + (self.w as i32 >> 1) - 16) as i16,
                    (self.y as i16 as i32 + (self.h as i32 >> 1) - 16) as i16,
                )
            };
            self.dead = true;
        }
    }

    pub fn draw(&self) {
        spr(self.spr).draw_part(self.x as i16 as i32, self.y as i16 as i32, self.srcx as i32, self.srcy as i32, self.w as i32, self.h as i32);
    }
}
impl_eyecandy!(EC_FloatingObject);

//------------------------------------------------------------------------------
// class EC_SoulsAward
//------------------------------------------------------------------------------
pub struct EC_SoulsAward {
    pub c_eyecandy: CEyecandy,
    spr: Ptr<gfxSprite>,
    spawnspr: Ptr<gfxSprite>,

    x: i16,
    y: i16,
    numSouls: i16,
    id: Vec<i16>,
    ttl: i16,
    timer: i16,
    count: i16,
    speed: f32,

    animationCount: i16,
    frame: i16,
    endmode: bool,
    w: i16,
    h: i16,
}
impl_base!(EC_SoulsAward => c_eyecandy: CEyecandy);

impl EC_SoulsAward {
    pub fn new(nspr: Ptr<gfxSprite>, nspr2: Ptr<gfxSprite>, nx: i16, ny: i16, timetolive: i16, nSpeed: f32, nSouls: i16, nSoulArray: &[i16]) -> Self {
        let mut this = EC_SoulsAward {
            c_eyecandy: CEyecandy::default(),
            spr: nspr,
            spawnspr: nspr2,
            x: nx,
            y: ny,
            ttl: timetolive,
            numSouls: nSouls,
            id: Vec::new(),
            timer: 0,
            count: 0,
            speed: 0.0,
            animationCount: 0,
            frame: 0,
            endmode: false,
            w: 0,
            h: 0,
        };

        if this.numSouls as i32 > MAXAWARDS {
            this.numSouls = MAXAWARDS as i16;
        }

        for k in 0..this.numSouls.max(0) as usize {
            this.id.push(nSoulArray[k]);
        }

        this.timer = 0;
        this.speed = nSpeed;
        this.count = 0;

        if this.numSouls < 1 {
            this.dead = true;
        }

        this.animationCount = 0;
        this.frame = 0;
        this.endmode = false;

        this.w = (nspr2.get_width() as i16 as i32 / 7) as i16;
        this.h = nspr2.get_height() as i16;
        this
    }

    pub fn update(&mut self) {
        if !self.endmode && {
            self.timer += 1;
            self.timer as f32 > self.speed
        } {
            self.timer = 0;

            let addangle = QUARTER_PI / 20.0f32;
            let startangle = -HALF_PI;

            let angle = (RANDOM_INT(21) - 10) as f32 * addangle + startangle;
            let velx = self.speed * angle.cos();
            let vely = self.speed * angle.sin();

            unsafe {
                let spr = Ptr::from_mut(&mut rm.spr_awardsouls);
                eyecandy[2].emplace(EC_RocketAward::new(
                    spr,
                    (self.x as i32 - 8) as i16,
                    (self.y as i32 - 8) as i16,
                    velx,
                    vely,
                    self.ttl,
                    self.id[self.count as usize],
                    0,
                    16,
                    16,
                    0,
                    0,
                ));
            }

            self.count += 1;
            if self.count >= self.numSouls {
                self.endmode = true;
                self.frame = 4;
            }
        }

        self.animationCount += 1;
        if self.animationCount > 6 {
            self.animationCount = 0;

            self.frame += 1;

            if !self.endmode && self.frame >= 4 {
                self.frame = 0;
            } else if self.endmode && self.frame >= 7 {
                self.dead = true;
            }
        }
    }

    pub fn draw(&self) {
        spr(self.spawnspr).draw_part(
            self.x as i32 - 16,
            self.y as i32 - 16,
            self.frame as i32 * self.w as i32,
            0,
            self.w as i32,
            self.h as i32,
        );
    }
}
impl_eyecandy!(EC_SoulsAward);

//------------------------------------------------------------------------------
// class EC_DoorFront
//------------------------------------------------------------------------------
pub struct EC_Door {
    pub c_eyecandy: CEyecandy,
    spr: Ptr<gfxSprite>,
    mariospr: Ptr<gfxSprite>,

    x: i16,
    y: i16,
    frame: i16,
    timer: i16,
    iw: i16,
    ih: i16,
    rate: i16,
    state: i16,
    offsetx: i16,
    offsety: i16,
    colorOffset: i16,
}
impl_base!(EC_Door => c_eyecandy: CEyecandy);

impl EC_Door {
    pub fn new(nspr: Ptr<gfxSprite>, nmariospr: Ptr<gfxSprite>, nx: i16, ny: i16, irate: i16, xOffset: i16, iColor: i16) -> Self {
        let ih: i16 = 32;
        EC_Door {
            c_eyecandy: CEyecandy::default(),
            spr: nspr,
            mariospr: nmariospr,
            x: nx,
            y: ny,
            timer: 0,
            rate: irate,
            iw: 32,
            ih,
            offsety: ih,
            state: 0,
            frame: 0,
            offsetx: xOffset,
            colorOffset: ((iColor as i32) << 5) as i16, // select which color door to use
        }
    }

    pub fn update(&mut self) {
        self.timer += 1;
        if self.timer > self.rate {
            self.timer = 0;

            if self.state == 0 {
                self.offsety -= 3;

                if self.offsety <= 0 {
                    self.state = 1;
                    self.offsety = 0;
                }

                self.frame = 1;
            } else if self.state == 1 {
                self.frame += 1;
                if self.frame >= 9 {
                    self.state = 2;
                    self.frame = 9;
                }
            } else if self.state == 2 {
                self.frame -= 1;
                if self.frame <= 1 {
                    self.state = 3;
                    self.frame = 1;
                }
            } else if self.state == 3 {
                self.offsety += 3;

                if self.offsety >= 32 {
                    self.dead = true;
                    self.offsety = 32;
                }

                self.frame = 1;
            }
        }
    }

    pub fn draw(&self) {
        let (x, y, iw, ih, offsety) = (self.x as i32, self.y as i32, self.iw as i32, self.ih as i32, self.offsety as i32);
        spr(self.spr).draw_part(x, y + offsety, 0, self.colorOffset as i32, iw, ih - offsety);

        if self.state == 1 {
            spr(self.mariospr).draw_part(
                x + 16 - HALFPW - PWOFFSET,
                y + 16 - HALFPH - PHOFFSET + offsety,
                self.offsetx as i32,
                0,
                iw,
                ih - offsety,
            );
        }

        spr(self.spr).draw_part(x, y + offsety, self.frame as i32 * iw, self.colorOffset as i32, iw, ih - offsety);
    }
}
impl_eyecandy!(EC_Door);

//------------------------------------------------------------------------------
// class EC_SuperStompExplosion
//------------------------------------------------------------------------------
pub struct EC_SuperStompExplosion {
    pub c_eyecandy: CEyecandy,
    spr: Ptr<gfxSprite>,
    ix: i16,
    iy: i16,
    iAnimationFrame: i16,
    iAnimationTimer: i16,
    iRate: i16,
}
impl_base!(EC_SuperStompExplosion => c_eyecandy: CEyecandy);

const fn r(x: i32, y: i32, w: i32, h: i32) -> SDL_Rect {
    SDL_Rect { x, y, w, h }
}

static rectSuperStompLeftSrc: [SDL_Rect; 8] =
    [r(0, 0, 48, 40), r(0, 40, 54, 42), r(0, 82, 48, 58), r(0, 140, 42, 70), r(0, 210, 44, 78), r(108, 0, 44, 82), r(96, 82, 42, 76), r(88, 158, 34, 74)];
static rectSuperStompRightSrc: [SDL_Rect; 8] =
    [r(48, 0, 48, 40), r(54, 40, 54, 42), r(48, 82, 48, 58), r(42, 140, 42, 70), r(44, 210, 44, 78), r(152, 0, 44, 82), r(138, 82, 42, 76), r(122, 158, 34, 74)];
static rectSuperStompLeftDst: [SDL_Rect; 8] = [
    r(-48, -40, 48, 40),
    r(-56, -42, 54, 42),
    r(-66, -58, 48, 58),
    r(-68, -70, 42, 70),
    r(-72, -78, 44, 78),
    r(-74, -82, 44, 82),
    r(-74, -76, 42, 76),
    r(-70, -74, 34, 74),
];
static rectSuperStompRightDst: [SDL_Rect; 8] =
    [r(0, -40, 48, 40), r(2, -42, 54, 42), r(18, -58, 48, 58), r(26, -70, 42, 70), r(28, -78, 44, 78), r(30, -82, 44, 82), r(32, -76, 42, 76), r(36, -74, 34, 74)];

impl EC_SuperStompExplosion {
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, irate: i16) -> Self {
        EC_SuperStompExplosion { c_eyecandy: CEyecandy::default(), spr: nspr, iRate: irate, ix: x, iy: y, iAnimationFrame: 0, iAnimationTimer: 0 }
    }

    pub fn update(&mut self) {
        self.iAnimationTimer += 1;
        if self.iAnimationTimer >= self.iRate {
            self.iAnimationTimer = 0;

            self.iAnimationFrame += 1;
            if self.iAnimationFrame > 7 {
                self.dead = true;
            }
        }
    }

    pub fn draw(&self) {
        if self.dead {
            return;
        }

        let f = self.iAnimationFrame as usize;
        spr(self.spr).draw_src(rectSuperStompLeftDst[f].x + self.ix as i32, rectSuperStompLeftDst[f].y + self.iy as i32, &rectSuperStompLeftSrc[f]);
        spr(self.spr).draw_src(rectSuperStompRightDst[f].x + self.ix as i32, rectSuperStompRightDst[f].y + self.iy as i32, &rectSuperStompRightSrc[f]);
    }
}
impl_eyecandy!(EC_SuperStompExplosion);

//------------------------------------------------------------------------------
// class eyecandy_container
//------------------------------------------------------------------------------
pub struct CEyecandyContainer {
    pub eyecandies: Vec<Box<dyn CEyecandyTrait>>,
    pub _alias: Aliased,
}

impl Default for CEyecandyContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl CEyecandyContainer {
    /// The C++ constructor only reserves MAXEYECANDY slots, which has no observable effect here.
    pub const fn new() -> Self {
        CEyecandyContainer { eyecandies: Vec::new(), _alias: Aliased::new() }
    }

    /// `emplace<T>(args...)`: the caller constructs `T` with the same arguments.
    pub fn emplace<T: CEyecandyTrait + 'static>(&mut self, ec: T) {
        self.eyecandies.push(Box::new(ec));
    }

    /// Like the C++ range-for, elements emplaced into this container during the loop are not visited.
    pub fn update(&mut self) {
        let this: *mut CEyecandyContainer = self;
        let n = self.eyecandies.len();
        for i in 0..n {
            let ec: *mut dyn CEyecandyTrait = unsafe { &mut *(&mut (*this).eyecandies)[i] };
            unsafe { (*ec).update() };
        }
    }

    pub fn draw(&self) {
        for ec in &self.eyecandies {
            ec.draw();
        }
    }

    pub fn clean(&mut self) {
        self.eyecandies.clear();
    }

    pub fn clean_dead_objects(&mut self) {
        self.eyecandies.retain(|ec| !ec.is_dead());
    }
}

static iSpotlightValues: [[i16; 4]; 8] =
    [[16, 8, 240, 96], [32, 16, 336, 80], [48, 24, 416, 64], [64, 32, 416, 0], [80, 40, 336, 0], [96, 48, 240, 0], [112, 56, 128, 0], [128, 64, 0, 0]];

pub struct Spotlight {
    ix: i16,
    iy: i16,
    iEndSize: i16,
    iTransparency: i16,
    iState: i16,
    fUpdated: bool,

    iSizeCounter: i16,
    iSize: i16,

    iHalfWidth: i16,
    iWidth: i16,
    rSrc: SDL_Rect,
    pub _alias: Aliased,
}

impl Spotlight {
    pub fn new(x: i16, y: i16, size: i16) -> Self {
        let iWidth = iSpotlightValues[0][0];
        Spotlight {
            ix: x,
            iy: y,
            iEndSize: size,
            iTransparency: 255,
            iState: 0,
            fUpdated: false,
            iSizeCounter: 0,
            iSize: 0,
            iWidth,
            iHalfWidth: iSpotlightValues[0][1],
            rSrc: SDL_Rect { x: iSpotlightValues[0][2] as i32, y: iSpotlightValues[0][3] as i32, w: iWidth as i32, h: iWidth as i32 },
            _alias: Aliased::new(),
        }
    }

    fn apply_size(&mut self) {
        let s = self.iSize as usize;
        self.iWidth = iSpotlightValues[s][0];
        self.iHalfWidth = iSpotlightValues[s][1];

        self.rSrc.x = iSpotlightValues[s][2] as i32;
        self.rSrc.y = iSpotlightValues[s][3] as i32;
        self.rSrc.w = self.iWidth as i32;
        self.rSrc.h = self.iWidth as i32;
    }

    pub fn update(&mut self) {
        // If the spotlight's position wasn't updated this frame, that means the parent is dead and
        // this spotlight needs to be removed
        if !self.fUpdated {
            self.iState = 2;
        }

        self.fUpdated = false;

        if self.iState == 0 {
            self.iSizeCounter += 1;
            if self.iSizeCounter >= 4 {
                self.iSizeCounter = 0;
                self.iSize += 1;
                if self.iSize >= self.iEndSize {
                    self.iSize = self.iEndSize;
                    self.iState = 1; // spotlight has reached it's full size
                }

                self.apply_size();
            }
        } else if self.iState == 2 {
            self.iSizeCounter += 1;
            if self.iSizeCounter >= 4 {
                self.iSizeCounter = 0;
                self.iSize -= 1;
                if self.iSize < 0 {
                    self.iSize = 0;
                    self.iState = 3; // stop drawing, spotlight is dead
                }

                self.apply_size();
            }
        }
    }

    pub fn update_position(&mut self, x: i16, y: i16) {
        self.ix = x;
        self.iy = y;

        self.fUpdated = true;
    }

    pub fn draw(&mut self) {
        unsafe {
            let (ix, iy, hw, w) = (self.ix as i32, self.iy as i32, self.iHalfWidth as i32, self.iWidth as i32);
            let rDst = SDL_Rect { x: ix - hw, y: iy - hw, w, h: w };
            rm.spr_overlayhole.draw_src_to(&self.rSrc, rm.spr_overlay.get_surface(), &rDst);

            if ix - hw < 0 {
                let rDstWrap = SDL_Rect { x: ix - hw + screenWidth, y: iy - hw, w, h: w };
                rm.spr_overlayhole.draw_src_to(&self.rSrc, rm.spr_overlay.get_surface(), &rDstWrap);
            } else if ix + hw >= screenWidth {
                let rDstWrap = SDL_Rect { x: ix - hw - screenWidth, y: iy - hw, w, h: w };
                rm.spr_overlayhole.draw_src_to(&self.rSrc, rm.spr_overlay.get_surface(), &rDstWrap);
            }
        }
    }

    pub fn is_dead(&self) -> bool {
        self.iState >= 3
    }
}

#[derive(Default)]
pub struct SpotlightManager {
    spotlightList: Vec<Ptr<Spotlight>>,
    pub _alias: Aliased,
}

impl SpotlightManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_spotlight(&mut self, ix: i16, iy: i16, iSize: i16) -> Ptr<Spotlight> {
        unsafe {
            if !game_values.spotlights {
                return Ptr::null();
            }
        }

        let s = Ptr::new_box(Spotlight::new(ix, iy, iSize));
        self.spotlightList.push(s);
        s
    }

    pub fn draw_spotlights(&mut self) {
        unsafe {
            // Clear the overlay surface again with black
            SDL_FillRect(rm.spr_overlay.get_surface(), null(), 0x0);
        }

        let mut i = 0;
        while i < self.spotlightList.len() {
            let mut s = self.spotlightList[i];
            s.update();

            if s.is_dead() {
                s.delete();
                self.spotlightList.remove(i);
            } else {
                s.draw();
                i += 1;
            }
        }

        // Draw the overlay
        unsafe { rm.spr_overlay.draw(0, 0) };
    }

    pub fn clear_spotlights(&mut self) {
        for s in self.spotlightList.drain(..) {
            s.delete();
        }
    }
}
