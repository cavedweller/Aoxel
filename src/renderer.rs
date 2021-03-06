// renderer.rs
// render in a unique task

extern mod glfw;
extern mod gl;
extern mod cgmath;


use std::mem;
use std::cast;
use std::ptr;
use std::vec;
use std::rand;
use std::str;

use std::rand::*;
use gl::types::*;
use gl::*;

use cgmath::projection;
use cgmath::matrix::*;
use cgmath::ptr::*;
use cgmath::angle::*;
use cgmath::quaternion::*;
use cgmath::point::*;
use cgmath::vector::*;

use chunk::*;
use chunk::Block;

use world::*;
use world::World;

use camera::*;
use camera::World;


// TODO extract into data file
// TODO change variables to a more rust style

static VS_SRC: &'static str =
  "#version 150\n\
  in vec3 position;\n\

  in vec3 color;\n\
  out vec3 Color;\n\

  uniform mat4 model_to_world;\n\
  uniform mat4 world_to_camera;\n\
  uniform mat4 camera_to_clip;\n\

  void main() {\n\
    Color = color;\n\
    vec4 camera_pos = vec4(position.x, position.y, position.z, 1.0);\n\
    gl_Position = model_to_world * camera_to_clip * world_to_camera * camera_pos;\n\
  }";

static FS_SRC: &'static str =
  "#version 150\n\
  in vec3 Color;\n\
  out vec4 outColor;\n\
  void main(){\n\
      outColor = vec4(Color.r/4, Color.r/4, Color.r/4, 1.0); \n\
  }";

pub struct Renderer {
  // OpenGL Buffers
  vao:      GLuint,
  vbo:      GLuint,
  ebo:      GLuint,
  program:  GLuint,

  model_to_world:   Mat4<f32>,
  world_to_camera:  Mat4<f32>,
  camera_to_clip:   Mat4<f32>,

  world: World

}


impl Renderer {
  pub fn set_world_to_camera(&mut self, view: Mat4<f32>) -> () {
    self.world_to_camera = view;
  }
  pub fn add_world(&mut self, world: World) -> () {
    self.world = world;
  }

  pub fn new() -> Renderer {
    let mut renderer = Renderer {
      vao:      0,
      vbo:      0,
      ebo:      0,
      program:  0,
      model_to_world:   Mat4::identity(),
//      world_to_camera:  Mat4::identity(),
      world_to_camera:  Mat4::look_at(&Point3::new(75.0 as f32, 75.0, 75.0),
                                      &Point3::new(0.0 as f32, 0.0, 0.0),
                                      &Vec3::new(0.0 as f32, 0.0, 1.0)),
      camera_to_clip:   Mat4::identity(),
      world: World::new()
    };


    gl::Enable(gl::DEPTH_TEST);
    //    TODO enable
    //gl::Enable(gl::CULL_FACE);

    // Compile Shaders
    let vs = compile_shader(VS_SRC, gl::VERTEX_SHADER);
    let fs = compile_shader(FS_SRC, gl::FRAGMENT_SHADER);

    renderer.program = gl::CreateProgram();

    // Make shaders
    gl::AttachShader(renderer.program, vs);
    gl::AttachShader(renderer.program, fs);

    unsafe {
      // Link Frag buffer
      "outColor".with_c_str(|ptr| gl::BindFragDataLocation(renderer.program, 0, ptr));
      gl::LinkProgram(renderer.program);

      // Vertice Array Object
      gl::GenVertexArrays(1, &mut renderer.vao);
      gl::BindVertexArray(renderer.vao);

      // Vertice Buffer Object
      gl::GenBuffers(1, &mut renderer.vbo);
      gl::BindBuffer(gl::ARRAY_BUFFER, renderer.vbo);

    // Use Shader
    gl::UseProgram(renderer.program);

    let pos_attr = "position".with_c_str(|ptr| gl::GetAttribLocation(renderer.program, ptr));
    gl::EnableVertexAttribArray(pos_attr as GLuint);
    gl::VertexAttribPointer(pos_attr as u32, 3, gl::BYTE, gl::FALSE,
                            (4 * mem::size_of::<GLbyte>()) as i32, ptr::null());

    let col_attr = "color".with_c_str(|ptr| gl::GetAttribLocation(renderer.program, ptr));
    gl::EnableVertexAttribArray(col_attr as GLuint);
    gl::VertexAttribPointer(col_attr as u32, 1, gl::BYTE, gl::FALSE,
                            (4 *mem::size_of::<GLbyte>()) as GLsizei,
                            cast::transmute(3*mem::size_of::<GLbyte>() as uint));

    }
    renderer
  }

  pub fn update(&mut self) {
    for x in range(0 as int, self.world.chunks.len() as int) {
      for y in range(0 as int, self.world.chunks.len() as int) {
        for z in range(0 as int, self.world.chunks.len() as int) {
          match self.world.chunks.find(&(x,y,z)) {
            None => (),
            Some(chunk) => {
              let mut block_vertexes: ~[GLbyte] = ~[];
              // loop over blocks in the chunk
              for z in range(0, chunk.len()) {
                for x in range(0, chunk.len()) {
                  for y in range(0, chunk.len()) {
                    match chunk.get_block(x, y, z) {
                      Some(block) =>  {
                        for i in gen_vertex(x, y, z, block, chunk).iter(){
                          block_vertexes.push(*i);
                        }
                        gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
                        unsafe{
                          gl::BufferData(gl::ARRAY_BUFFER,
                          (block_vertexes.len() * mem::size_of::<GLbyte>()) as GLsizeiptr,
                          cast::transmute(&block_vertexes[0]), gl::STATIC_DRAW);
                        }
                      }
                      None => ()
                    }
                  }
                }
              }
//              self.world_to_camera  = Mat4::look_at(&Point3::new(75.0 as f32, 75.0, 75.0),
//                                                    &Point3::new(0.0 as f32, 0.0, 0.0),
//                                                    &Vec3::new(0.0 as f32, 0.0, 1.0));
              self.camera_to_clip   = projection::perspective(deg(45.0 as f32),
                                                              800.0/600.0, 1.0, 150.0);


              unsafe {
                let uni_m_to_wor =
                  "model_to_world".with_c_str(|ptr| gl::GetUniformLocation(self.program, ptr));
                let uni_w_to_cam =
                  "world_to_camera".with_c_str(|ptr| gl::GetUniformLocation(self.program, ptr));
                let uni_cam_to_c =
                  "camera_to_clip".with_c_str(|ptr| gl::GetUniformLocation(self.program, ptr));

                gl::UniformMatrix4fv(uni_m_to_wor, 1, gl::FALSE, self.model_to_world.ptr());
                gl::UniformMatrix4fv(uni_w_to_cam, 1, gl::FALSE, self.world_to_camera.ptr());
                gl::UniformMatrix4fv(uni_cam_to_c, 1, gl::FALSE, self.camera_to_clip.ptr());

                // Draw to the screen
                gl::DrawArrays(gl::TRIANGLES, 0, block_vertexes.len() as i32);

              }
            }
          }
        }
      }
    }
  }
}

fn compile_shader(src: &str, ty: GLenum) -> GLuint {
  let shader = gl::CreateShader(ty);
  unsafe {
    // grab pointer for shader
    src.with_c_str(|ptr| gl::ShaderSource(shader, 1, &ptr, ptr::null()));
    gl::CompileShader(shader);

    // compile status
    let mut status = gl::FALSE as GLint;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

    if status != (gl::TRUE as GLint) {
      let mut len = 0;
      gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
      let mut buf = vec::from_elem(len as uint, 0u8);
      gl::GetShaderInfoLog(shader, len, ptr::mut_null(), buf.as_mut_ptr() as *mut GLchar);
      print!("{}", str::raw::from_utf8(buf));
      fail!();
    }
  }
  shader
}


fn gen_vertex(x_in: int, y_in: int, z_in: int, block_type: Block, chunk: &Chunk) -> ~[GLbyte] {
  let (coord_x, coord_y, coord_z) = chunk.coords;

  let x: i8 = (x_in + coord_x*chunk.size) as i8;
  let y: i8 = (y_in + coord_y*chunk.size) as i8;
  let z: i8 = (z_in + coord_z*chunk.size) as i8;
  let block_type: i8 = block_type as i8;

  let mut build_vec: ~[GLbyte] = ~[];

  if chunk.get_block(x_in - 1, y_in , z_in).is_none() {
    build_vec = vec::append(build_vec,
                           [x,      y,      z,            block_type,
                            x,      y,      z + 1,        block_type,
                            x,      y + 1,  z,            block_type,
                            x,      y + 1,  z,            block_type,
                            x,      y,      z + 1,        block_type,
                            x,      y + 1,  z + 1,        block_type]);
  }

    // View from positive x
  if chunk.get_block(x_in + 1, y_in , z_in).is_none() {
    build_vec = vec::append(build_vec,
                           [x + 1,  y,      z,            block_type,
                            x + 1,  y + 1,  z,            block_type,
                            x + 1,  y,      z + 1,        block_type,
                            x + 1,  y + 1,  z,            block_type,
                            x + 1,  y + 1,  z + 1,        block_type,
                            x + 1,  y,      z + 1,        block_type]);
  }

    // View from negative y
  if chunk.get_block(x_in, y_in - 1, z_in).is_none() {
    build_vec = vec::append(build_vec,
                           [x,      y,      z,            block_type,
                            x + 1,  y,      z,            block_type,
                            x + 1,  y,      z + 1,        block_type,
                            x + 1,  y,      z + 1,        block_type,
                            x,      y,      z + 1,        block_type,
                            x,      y,      z,            block_type]);
  }

    // View from positive y
  if chunk.get_block(x_in, y_in + 1, z_in).is_none() {
    build_vec = vec::append(build_vec,
                           [x,      y + 1,  z,            block_type,
                            x + 1,  y + 1,  z,            block_type,
                            x + 1,  y + 1,  z + 1,        block_type,
                            x + 1,  y + 1,  z + 1,        block_type,
                            x,      y + 1,  z + 1,        block_type,
                            x,      y + 1,  z,            block_type]);
  }

    // View from negative z
  if chunk.get_block(x_in, y_in, z_in - 1).is_none() {
    build_vec = vec::append(build_vec,
                           [x,      y,      z + 1,        block_type,
                            x,      y + 1,  z + 1,        block_type,
                            x + 1,  y,      z + 1,        block_type,
                            x + 1,  y,      z + 1,        block_type,
                            x,      y + 1,  z + 1,        block_type,
                            x + 1,  y + 1,  z + 1,        block_type]);
  }
    // View from positive z
  if chunk.get_block(x_in, y_in, z_in + 1).is_none() {
    build_vec = vec::append(build_vec,
                           [x,      y,      z,            block_type,
                            x + 1,  y,      z,            block_type,
                            x,      y + 1,  z,            block_type,
                            x + 1,  y,      z,            block_type,
                            x + 1,  y + 1,  z,            block_type,
                            x,      y + 1,  z,            block_type]);
  }
  build_vec
}
