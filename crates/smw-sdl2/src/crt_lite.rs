//! The built-in presenter for platforms librashader does not reach (OpenGL ES 2 on Android, WebGL 1 in browsers):
//! GLSL ports of EasyMode's CRT-Easymode and Timothy Lottes' CRT shader, drawn straight to the window.

use crate::gl_common::{drawable_size, letterbox, load_gl, save_shot, shot_frame, watch_mouse};
use crate::shader_overlay::{link_program, Glsl, OverlayGl};
use glow::HasContext;
use sdl2::sys::*;
use smw_platform::{Frame, Overlay, SCREEN_H, SCREEN_W};
use std::ffi::CStr;

/// The built-in presets, as `set_shader` names them (`builtin:<name>`) and the menu shows them.
pub const BUILTIN: [&str; 2] = ["CRT Easymode", "CRT Lottes"];

const VERTEX: &str = "attribute vec2 pos;
varying vec2 vTexCoord;
void main() {
    vTexCoord = vec2(pos.x * 0.5 + 0.5, 0.5 - pos.y * 0.5);
    gl_Position = vec4(pos, 0.0, 1.0);
}
";

// CRT-Easymode by EasyMode (libretro glsl-shaders, GPL), with its default parameters.
// Scanlines need two output rows per source line, so they are off below that.
const EASYMODE: &str = "uniform sampler2D Source;
uniform vec4 SourceSize;
uniform vec2 OutputSize;
varying vec2 vTexCoord;

#define SHARPNESS_H 0.5
#define SHARPNESS_V 1.0
#define MASK_STRENGTH 0.3
#define MASK_DOT_WIDTH 1.0
#define MASK_DOT_HEIGHT 1.0
#define MASK_STAGGER 0.0
#define MASK_SIZE 1.0
#define SCANLINE_STRENGTH 1.0
#define SCANLINE_BEAM_WIDTH_MIN 1.5
#define SCANLINE_BEAM_WIDTH_MAX 1.5
#define SCANLINE_BRIGHT_MIN 0.35
#define SCANLINE_BRIGHT_MAX 0.65
#define GAMMA_INPUT 2.0
#define GAMMA_OUTPUT 1.8
#define BRIGHT_BOOST 1.2
#define DILATION 1.0
#define FIX(c) max(abs(c), 1e-5)
#define PI 3.141592653589

vec4 tex2d(vec2 c) {
    // The frame's B, G, R, A bytes arrive as R, G, B, A.
    vec4 col = vec4(texture2D(Source, c).bgr, 1.0);
    return col * mix(vec4(1.0), col, DILATION);
}

float curve_distance(float x, float sharp) {
    float x_step = step(0.5, x);
    float curve = 0.5 - sqrt(0.25 - (x - x_step) * (x - x_step)) * sign(0.5 - x);
    return mix(x, curve, sharp);
}

vec3 filter_lanczos(vec4 coeffs, vec2 co, vec2 dx) {
    mat4 m = mat4(tex2d(co - dx), tex2d(co), tex2d(co + dx), tex2d(co + 2.0 * dx));
    vec4 col = m * coeffs;
    return clamp(col, min(m[1], m[2]), max(m[1], m[2])).rgb;
}

void main() {
    vec2 dx = vec2(SourceSize.z, 0.0);
    vec2 dy = vec2(0.0, SourceSize.w);
    vec2 pix_co = vTexCoord * SourceSize.xy - vec2(0.5, 0.5);
    vec2 tex_co = (floor(pix_co) + vec2(0.5, 0.5)) * SourceSize.zw;
    vec2 dist = fract(pix_co);

    float curve_x = curve_distance(dist.x, SHARPNESS_H * SHARPNESS_H);
    vec4 coeffs = PI * vec4(1.0 + curve_x, curve_x, 1.0 - curve_x, 2.0 - curve_x);
    coeffs = FIX(coeffs);
    coeffs = 2.0 * sin(coeffs) * sin(coeffs / 2.0) / (coeffs * coeffs);
    coeffs /= dot(coeffs, vec4(1.0));
    vec3 col = filter_lanczos(coeffs, tex_co, dx);
    vec3 col2 = filter_lanczos(coeffs, tex_co + dy, dx);

    col = mix(col, col2, curve_distance(dist.y, SHARPNESS_V));
    col = pow(col, vec3(GAMMA_INPUT / (DILATION + 1.0)));

    float luma = dot(vec3(0.2126, 0.7152, 0.0722), col);
    float bright = (max(col.r, max(col.g, col.b)) + luma) / 2.0;
    float scan_bright = clamp(bright, SCANLINE_BRIGHT_MIN, SCANLINE_BRIGHT_MAX);
    float scan_beam = clamp(bright * SCANLINE_BEAM_WIDTH_MAX, SCANLINE_BEAM_WIDTH_MIN, SCANLINE_BEAM_WIDTH_MAX);
    float scan_weight = 1.0 - pow(cos(vTexCoord.y * 2.0 * PI * SourceSize.y) * 0.5 + 0.5, scan_beam) * SCANLINE_STRENGTH;
    if (OutputSize.y < SourceSize.y * 2.0) {
        scan_weight = 1.0;
    }

    float mask = 1.0 - MASK_STRENGTH;
    vec2 mod_fac = floor(vTexCoord * OutputSize / vec2(MASK_SIZE, MASK_DOT_HEIGHT * MASK_SIZE));
    float dot_no = floor(mod((mod_fac.x + mod(mod_fac.y, 2.0) * MASK_STAGGER) / MASK_DOT_WIDTH, 3.0));
    vec3 mask_weight = dot_no < 0.5 ? vec3(1.0, mask, mask) : (dot_no < 1.5 ? vec3(mask, 1.0, mask) : vec3(mask, mask, 1.0));

    col2 = col;
    col *= vec3(scan_weight);
    col = mix(col, col2, scan_bright);
    col *= mask_weight;
    col = pow(col, vec3(1.0 / GAMMA_OUTPUT));

    gl_FragColor = vec4(col * BRIGHT_BOOST, 1.0);
}
";

// CRT Lottes by Timothy Lottes (libretro slang-shaders, public domain), with its default parameters.
const LOTTES: &str = "uniform sampler2D Source;
uniform vec4 SourceSize;
uniform vec2 OutputSize;
varying vec2 vTexCoord;

#define HARD_SCAN -8.0
#define HARD_PIX -3.0
#define WARP_X 0.031
#define WARP_Y 0.041
#define MASK_DARK 0.5
#define MASK_LIGHT 1.5
#define BRIGHT_BOOST 1.0
#define HARD_BLOOM_PIX -1.5
#define HARD_BLOOM_SCAN -2.0
#define BLOOM_AMOUNT 0.15

float to_linear1(float c) {
    return c <= 0.04045 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4);
}

vec3 to_linear(vec3 c) {
    return vec3(to_linear1(c.r), to_linear1(c.g), to_linear1(c.b));
}

float to_srgb1(float c) {
    return c < 0.0031308 ? c * 12.92 : 1.055 * pow(c, 0.41666) - 0.055;
}

vec3 to_srgb(vec3 c) {
    return vec3(to_srgb1(c.r), to_srgb1(c.g), to_srgb1(c.b));
}

vec3 fetch(vec2 pos, vec2 off) {
    pos = (floor(pos * SourceSize.xy + off) + vec2(0.5, 0.5)) * SourceSize.zw;
    // The frame's B, G, R, A bytes arrive as R, G, B, A.
    return to_linear(BRIGHT_BOOST * texture2D(Source, pos).bgr);
}

vec2 dist(vec2 pos) {
    pos = pos * SourceSize.xy;
    return -((pos - floor(pos)) - vec2(0.5));
}

// The preset's kernel shape is 2.0, so the original's pow(abs(pos), shape) is a square.
float gaus(float pos, float scale) {
    return exp2(scale * pos * pos);
}

vec3 horz3(vec2 pos, float off) {
    vec3 b = fetch(pos, vec2(-1.0, off));
    vec3 c = fetch(pos, vec2(0.0, off));
    vec3 d = fetch(pos, vec2(1.0, off));
    float dst = dist(pos).x;
    float wb = gaus(dst - 1.0, HARD_PIX);
    float wc = gaus(dst, HARD_PIX);
    float wd = gaus(dst + 1.0, HARD_PIX);
    return (b * wb + c * wc + d * wd) / (wb + wc + wd);
}

vec3 horz5(vec2 pos, float off, float scale) {
    vec3 a = fetch(pos, vec2(-2.0, off));
    vec3 b = fetch(pos, vec2(-1.0, off));
    vec3 c = fetch(pos, vec2(0.0, off));
    vec3 d = fetch(pos, vec2(1.0, off));
    vec3 e = fetch(pos, vec2(2.0, off));
    float dst = dist(pos).x;
    float wa = gaus(dst - 2.0, scale);
    float wb = gaus(dst - 1.0, scale);
    float wc = gaus(dst, scale);
    float wd = gaus(dst + 1.0, scale);
    float we = gaus(dst + 2.0, scale);
    return (a * wa + b * wb + c * wc + d * wd + e * we) / (wa + wb + wc + wd + we);
}

vec3 horz7(vec2 pos, float off) {
    vec3 a = fetch(pos, vec2(-3.0, off));
    vec3 b = fetch(pos, vec2(-2.0, off));
    vec3 c = fetch(pos, vec2(-1.0, off));
    vec3 d = fetch(pos, vec2(0.0, off));
    vec3 e = fetch(pos, vec2(1.0, off));
    vec3 f = fetch(pos, vec2(2.0, off));
    vec3 g = fetch(pos, vec2(3.0, off));
    float dst = dist(pos).x;
    float wa = gaus(dst - 3.0, HARD_BLOOM_PIX);
    float wb = gaus(dst - 2.0, HARD_BLOOM_PIX);
    float wc = gaus(dst - 1.0, HARD_BLOOM_PIX);
    float wd = gaus(dst, HARD_BLOOM_PIX);
    float we = gaus(dst + 1.0, HARD_BLOOM_PIX);
    float wf = gaus(dst + 2.0, HARD_BLOOM_PIX);
    float wg = gaus(dst + 3.0, HARD_BLOOM_PIX);
    return (a * wa + b * wb + c * wc + d * wd + e * we + f * wf + g * wg) / (wa + wb + wc + wd + we + wf + wg);
}

float scan(vec2 pos, float off, float scale) {
    return gaus(dist(pos).y + off, scale);
}

vec3 tri(vec2 pos) {
    vec3 a = horz3(pos, -1.0);
    vec3 b = horz5(pos, 0.0, HARD_PIX);
    vec3 c = horz3(pos, 1.0);
    return a * scan(pos, -1.0, HARD_SCAN) + b * scan(pos, 0.0, HARD_SCAN) + c * scan(pos, 1.0, HARD_SCAN);
}

vec3 bloom(vec2 pos) {
    vec3 a = horz5(pos, -2.0, HARD_BLOOM_PIX);
    vec3 b = horz7(pos, -1.0);
    vec3 c = horz7(pos, 0.0);
    vec3 d = horz7(pos, 1.0);
    vec3 e = horz5(pos, 2.0, HARD_BLOOM_PIX);
    return a * scan(pos, -2.0, HARD_BLOOM_SCAN) + b * scan(pos, -1.0, HARD_BLOOM_SCAN) + c * scan(pos, 0.0, HARD_BLOOM_SCAN)
        + d * scan(pos, 1.0, HARD_BLOOM_SCAN) + e * scan(pos, 2.0, HARD_BLOOM_SCAN);
}

vec2 warp(vec2 pos) {
    pos = pos * 2.0 - 1.0;
    pos *= vec2(1.0 + (pos.y * pos.y) * WARP_X, 1.0 + (pos.x * pos.x) * WARP_Y);
    return pos * 0.5 + 0.5;
}

// The preset's stretched VGA shadow mask (shadowMask 3).
vec3 mask(vec2 pos) {
    vec3 m = vec3(MASK_DARK);
    float x = fract((pos.x + pos.y * 3.0) * 0.166666666);
    if (x < 0.333) {
        m.r = MASK_LIGHT;
    } else if (x < 0.666) {
        m.g = MASK_LIGHT;
    } else {
        m.b = MASK_LIGHT;
    }
    return m;
}

void main() {
    vec2 pos = warp(vTexCoord);
    vec3 col = tri(pos) + bloom(pos) * BLOOM_AMOUNT;
    col *= mask(vTexCoord * OutputSize * 1.000001);
    if (pos.x <= 0.0001 || pos.x >= 0.9999 || pos.y <= 0.0001 || pos.y >= 0.9999) {
        col = vec3(0.0);
    }
    gl_FragColor = vec4(to_srgb(col), 1.0);
}
";

/// The language the built-in presenter's context runs.
fn glsl() -> Glsl {
    if cfg!(any(target_os = "android", target_os = "emscripten")) {
        Glsl::Es100
    } else {
        Glsl::Legacy120
    }
}

/// Asks for OpenGL ES 2 (WebGL 1 in browsers) on Android and the web, and the default context elsewhere.
pub unsafe fn request_context() {
    if glsl() == Glsl::Es100 {
        SDL_GL_SetAttribute(SDL_GLattr::SDL_GL_CONTEXT_PROFILE_MASK, SDL_GLprofile::SDL_GL_CONTEXT_PROFILE_ES as i32);
        SDL_GL_SetAttribute(SDL_GLattr::SDL_GL_CONTEXT_MAJOR_VERSION, 2);
        SDL_GL_SetAttribute(SDL_GLattr::SDL_GL_CONTEXT_MINOR_VERSION, 0);
    }
}

pub struct CrtLite {
    context: SDL_GLContext,
    gl: glow::Context,
    program: glow::Program,
    vbo: glow::Buffer,
    input: glow::Texture,
    output_size: Option<glow::UniformLocation>,
    overlay: OverlayGl,
    frame_count: usize,
    shot: Option<usize>,
}

impl CrtLite {
    /// Safety: `window` was created with `SDL_WINDOW_OPENGL` after `request_context`.
    pub unsafe fn new(window: *mut SDL_Window, name: &str) -> Result<Self, String> {
        if !BUILTIN.contains(&name) {
            return Err(format!("no built-in shader named {name}"));
        }
        let context = SDL_GL_CreateContext(window);
        if context.is_null() {
            return Err(format!("no OpenGL context: {}", CStr::from_ptr(SDL_GetError()).to_string_lossy()));
        }
        match Self::with_context(context, if name == "CRT Lottes" { LOTTES } else { EASYMODE }) {
            Ok(lite) => {
                println!("[gfx] Built-in shader {name} on OpenGL {}", lite.gl.get_parameter_string(glow::VERSION));
                Ok(lite)
            }
            Err(e) => {
                SDL_GL_DeleteContext(context);
                Err(e)
            }
        }
    }

    unsafe fn with_context(context: SDL_GLContext, fragment: &str) -> Result<Self, String> {
        SDL_GL_SetSwapInterval(if smw_platform::want_vsync() { 1 } else { 0 });
        let gl = load_gl();
        // A WebGL canvas keeps its one context, so the renderer's vertex arrays can still be enabled.
        for i in 0..gl.get_parameter_i32(glow::MAX_VERTEX_ATTRIBS) as u32 {
            gl.disable_vertex_attrib_array(i);
        }
        let program = link_program(&gl, glsl(), VERTEX, fragment)?;
        let es = glsl() == Glsl::Es100;
        let input = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(input));
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
        // WebGL 1 samples a 640x480 texture only with edge clamping.
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
        let internal = if es { glow::RGBA } else { glow::RGBA8 };
        let (w, h) = (SCREEN_W as i32, SCREEN_H as i32);
        gl.tex_image_2d(glow::TEXTURE_2D, 0, internal as i32, w, h, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(Option::None));

        let quad: [f32; 8] = [-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        let vbo = gl.create_buffer()?;
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        let bytes = std::slice::from_raw_parts(quad.as_ptr() as *const u8, std::mem::size_of_val(&quad));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);

        gl.use_program(Some(program));
        gl.uniform_1_i32(gl.get_uniform_location(program, "Source").as_ref(), 0);
        let source_size = gl.get_uniform_location(program, "SourceSize");
        gl.uniform_4_f32(source_size.as_ref(), w as f32, h as f32, 1.0 / w as f32, 1.0 / h as f32);
        let output_size = gl.get_uniform_location(program, "OutputSize");
        let overlay = OverlayGl::new(&gl, glsl())?;
        watch_mouse(true);
        Ok(CrtLite { context, gl, program, vbo, input, output_size, overlay, frame_count: 0, shot: shot_frame() })
    }

    pub fn vsync(&self) -> bool {
        // SAFETY: takes no pointers.
        unsafe { SDL_GL_GetSwapInterval() != 0 }
    }

    /// Safety: `window` is the window `new` was given.
    /// Returns how long the frame upload took.
    pub unsafe fn present(&mut self, window: *mut SDL_Window, frame: Frame<'_>, overlay: Option<&Overlay>) -> std::time::Duration {
        let gl = &self.gl;
        let (dw, dh) = drawable_size(window);
        let (scale, ox, oy) = letterbox(dw, dh);
        let (vw, vh) = ((SCREEN_W as f32 * scale) as i32, (SCREEN_H as f32 * scale) as i32);
        gl.bind_framebuffer(glow::FRAMEBUFFER, Option::None);
        gl.viewport(0, 0, dw, dh);
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
        // The viewport's origin is the window's bottom-left corner.
        gl.viewport(ox as i32, (dh as f32 - oy) as i32 - vh, vw, vh);
        gl.use_program(Some(self.program));
        gl.uniform_2_f32(self.output_size.as_ref(), vw as f32, vh as f32);
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.input));
        let bytes = std::slice::from_raw_parts(frame.pixels.as_ptr() as *const u8, std::mem::size_of_val(frame.pixels));
        let (w, h) = (SCREEN_W as i32, SCREEN_H as i32);
        let upload = std::time::Instant::now();
        gl.tex_sub_image_2d(glow::TEXTURE_2D, 0, 0, 0, w, h, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(Some(bytes)));
        let upload = upload.elapsed();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
        gl.enable_vertex_attrib_array(0);
        gl.disable_vertex_attrib_array(1);
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        if let Some(overlay) = overlay {
            self.overlay.draw(gl, overlay, dw, dh);
        }
        if self.shot == Some(self.frame_count) {
            save_shot(gl, dw, dh, self.frame_count);
        }
        self.frame_count += 1;
        SDL_GL_SwapWindow(window);
        upload
    }
}

impl Drop for CrtLite {
    fn drop(&mut self) {
        // SAFETY: the watch and the context were made in `new` and are removed once.
        unsafe {
            watch_mouse(false);
            SDL_GL_DeleteContext(self.context);
        }
    }
}
