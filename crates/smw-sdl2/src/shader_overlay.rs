//! The overlay (touch controls) for the OpenGL presenters: textured quads in window pixels, alpha blended.

use glow::HasContext;
use smw_platform::Overlay;

/// The shading language a presenter's context runs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Glsl {
    /// OpenGL 3.3 core, which needs a vertex array object to draw.
    Core330,
    /// OpenGL 2.1 on desktops.
    Legacy120,
    /// OpenGL ES 2 and WebGL 1.
    Es100,
}

impl Glsl {
    /// The lines that start a shader of this language: the version, and for ES a default float precision.
    pub fn prelude(self) -> &'static str {
        match self {
            Glsl::Core330 => "#version 330\n",
            Glsl::Legacy120 => "#version 120\n",
            Glsl::Es100 => "#version 100\n#ifdef GL_FRAGMENT_PRECISION_HIGH\nprecision highp float;\n#else\nprecision mediump float;\n#endif\n",
        }
    }
}

const VERTEX_330: &str = "in vec2 pos;
in vec2 uv;
uniform vec2 screen;
out vec2 v_uv;
void main() {
    v_uv = uv;
    gl_Position = vec4(pos.x / screen.x * 2.0 - 1.0, 1.0 - pos.y / screen.y * 2.0, 0.0, 1.0);
}
";

const FRAGMENT_330: &str = "in vec2 v_uv;
uniform sampler2D image;
uniform vec4 tint;
out vec4 color;
void main() {
    color = texture(image, v_uv) * tint;
}
";

const VERTEX_100: &str = "attribute vec2 pos;
attribute vec2 uv;
uniform vec2 screen;
varying vec2 v_uv;
void main() {
    v_uv = uv;
    gl_Position = vec4(pos.x / screen.x * 2.0 - 1.0, 1.0 - pos.y / screen.y * 2.0, 0.0, 1.0);
}
";

const FRAGMENT_100: &str = "varying vec2 v_uv;
uniform sampler2D image;
uniform vec4 tint;
void main() {
    gl_FragColor = texture2D(image, v_uv) * tint;
}
";

/// Compiles and links `vertex` and `fragment`, each after `glsl`'s prelude, with `pos` at attribute 0 and `uv` at 1.
pub unsafe fn link_program(gl: &glow::Context, glsl: Glsl, vertex: &str, fragment: &str) -> Result<glow::Program, String> {
    let program = gl.create_program()?;
    for (kind, src) in [(glow::VERTEX_SHADER, vertex), (glow::FRAGMENT_SHADER, fragment)] {
        let shader = gl.create_shader(kind)?;
        gl.shader_source(shader, &format!("{}{src}", glsl.prelude()));
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            return Err(format!("shader: {}", gl.get_shader_info_log(shader)));
        }
        gl.attach_shader(program, shader);
        gl.delete_shader(shader);
    }
    gl.bind_attrib_location(program, 0, "pos");
    gl.bind_attrib_location(program, 1, "uv");
    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        return Err(format!("program: {}", gl.get_program_info_log(program)));
    }
    Ok(program)
}

pub struct OverlayGl {
    program: glow::Program,
    glsl: Glsl,
    /// Core contexts draw through a vertex array; ES 2 and WebGL 1 have none.
    vao: Option<glow::VertexArray>,
    vbo: glow::Buffer,
    screen: Option<glow::UniformLocation>,
    tint: Option<glow::UniformLocation>,
    /// The `Overlay::generation` that `textures` were made from.
    generation: Option<u64>,
    textures: Vec<glow::Texture>,
}

impl OverlayGl {
    pub unsafe fn new(gl: &glow::Context, glsl: Glsl) -> Result<Self, String> {
        let (vertex, fragment) = if glsl == Glsl::Core330 { (VERTEX_330, FRAGMENT_330) } else { (VERTEX_100, FRAGMENT_100) };
        let program = link_program(gl, glsl, vertex, fragment).map_err(|e| format!("overlay {e}"))?;
        let vbo = gl.create_buffer()?;
        let vao = if glsl == Glsl::Core330 {
            let vao = gl.create_vertex_array()?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            set_attributes(gl);
            gl.bind_vertex_array(Option::None);
            Some(vao)
        } else {
            Option::None
        };
        let screen = gl.get_uniform_location(program, "screen");
        let tint = gl.get_uniform_location(program, "tint");
        gl.use_program(Some(program));
        gl.uniform_1_i32(gl.get_uniform_location(program, "image").as_ref(), 0);
        gl.use_program(Option::None);
        Ok(OverlayGl { program, glsl, vao, vbo, screen, tint, generation: Option::None, textures: Vec::new() })
    }

    /// Uploads the overlay's images when its generation changes.
    unsafe fn upload(&mut self, gl: &glow::Context, overlay: &Overlay) -> Result<(), String> {
        if self.generation == Some(overlay.generation) {
            return Ok(());
        }
        for tex in self.textures.drain(..) {
            gl.delete_texture(tex);
        }
        for image in overlay.images.iter() {
            let tex = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
            if self.glsl != Glsl::Es100 {
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAX_LEVEL, 0);
            }
            // ES 2 and WebGL 1 take only an unsized internal format equal to the data's.
            let internal = if self.glsl == Glsl::Es100 { glow::RGBA } else { glow::RGBA8 };
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                internal as i32,
                image.w as i32,
                image.h as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&image.rgba)),
            );
            self.textures.push(tex);
        }
        self.generation = Some(overlay.generation);
        Ok(())
    }

    /// Draws into the bound framebuffer, which is `w` by `h` pixels.
    pub unsafe fn draw(&mut self, gl: &glow::Context, overlay: &Overlay, w: i32, h: i32) {
        if let Err(e) = self.upload(gl, overlay) {
            eprintln!("[gfx] Overlay textures: {e}");
            return;
        }
        gl.viewport(0, 0, w, h);
        gl.use_program(Some(self.program));
        gl.uniform_2_f32(self.screen.as_ref(), w as f32, h as f32);
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
        match self.vao {
            Some(vao) => gl.bind_vertex_array(Some(vao)),
            None => set_attributes(gl),
        }
        gl.active_texture(glow::TEXTURE0);
        gl.enable(glow::BLEND);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        for q in &overlay.quads {
            let Some(&tex) = self.textures.get(q.image as usize) else { continue };
            let (x0, y0, x1, y1) = (q.x as f32, q.y as f32, (q.x + q.w) as f32, (q.y + q.h) as f32);
            let verts: [f32; 16] = [x0, y0, 0.0, 0.0, x1, y0, 1.0, 0.0, x0, y1, 0.0, 1.0, x1, y1, 1.0, 1.0];
            let bytes = std::slice::from_raw_parts(verts.as_ptr() as *const u8, std::mem::size_of_val(&verts));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STREAM_DRAW);
            let [r, g, b] = q.color.map(|c| c as f32 / 255.0);
            gl.uniform_4_f32(self.tint.as_ref(), r, g, b, q.alpha as f32 / 255.0);
            gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
        gl.disable(glow::BLEND);
        if self.vao.is_some() {
            gl.bind_vertex_array(Option::None);
        }
        gl.use_program(Option::None);
    }
}

/// Points attributes 0 (`pos`) and 1 (`uv`) at the bound buffer's interleaved x, y, u, v floats.
unsafe fn set_attributes(gl: &glow::Context) {
    let stride = 4 * std::mem::size_of::<f32>() as i32;
    gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
    gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 2 * std::mem::size_of::<f32>() as i32);
    gl.enable_vertex_attrib_array(0);
    gl.enable_vertex_attrib_array(1);
}
