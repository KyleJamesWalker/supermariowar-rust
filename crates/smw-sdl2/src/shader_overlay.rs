//! The overlay (touch controls) for the OpenGL presenter: textured quads in window pixels, alpha blended.

use glow::HasContext;
use smw_platform::Overlay;

const VERTEX: &str = "#version 330
in vec2 pos;
in vec2 uv;
uniform vec2 screen;
out vec2 v_uv;
void main() {
    v_uv = uv;
    gl_Position = vec4(pos.x / screen.x * 2.0 - 1.0, 1.0 - pos.y / screen.y * 2.0, 0.0, 1.0);
}
";

const FRAGMENT: &str = "#version 330
in vec2 v_uv;
uniform sampler2D image;
uniform vec4 tint;
out vec4 color;
void main() {
    color = texture(image, v_uv) * tint;
}
";

pub struct OverlayGl {
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    screen: Option<glow::UniformLocation>,
    tint: Option<glow::UniformLocation>,
    /// The `Overlay::generation` that `textures` were made from.
    generation: Option<u64>,
    textures: Vec<glow::Texture>,
}

impl OverlayGl {
    pub unsafe fn new(gl: &glow::Context) -> Result<Self, String> {
        let program = gl.create_program()?;
        for (kind, src) in [(glow::VERTEX_SHADER, VERTEX), (glow::FRAGMENT_SHADER, FRAGMENT)] {
            let shader = gl.create_shader(kind)?;
            gl.shader_source(shader, src);
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                return Err(format!("overlay shader: {}", gl.get_shader_info_log(shader)));
            }
            gl.attach_shader(program, shader);
            gl.delete_shader(shader);
        }
        gl.bind_attrib_location(program, 0, "pos");
        gl.bind_attrib_location(program, 1, "uv");
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            return Err(format!("overlay program: {}", gl.get_program_info_log(program)));
        }
        let vao = gl.create_vertex_array()?;
        let vbo = gl.create_buffer()?;
        gl.bind_vertex_array(Some(vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        let stride = 4 * std::mem::size_of::<f32>() as i32;
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
        gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 2 * std::mem::size_of::<f32>() as i32);
        gl.enable_vertex_attrib_array(0);
        gl.enable_vertex_attrib_array(1);
        gl.bind_vertex_array(Option::None);
        let screen = gl.get_uniform_location(program, "screen");
        let tint = gl.get_uniform_location(program, "tint");
        gl.use_program(Some(program));
        gl.uniform_1_i32(gl.get_uniform_location(program, "image").as_ref(), 0);
        gl.use_program(Option::None);
        Ok(OverlayGl { program, vao, vbo, screen, tint, generation: Option::None, textures: Vec::new() })
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
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAX_LEVEL, 0);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
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
        gl.bind_vertex_array(Some(self.vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
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
        gl.bind_vertex_array(Option::None);
        gl.use_program(Option::None);
    }
}
