struct Globals {
  res: u32,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

@group(0) @binding(1)
var texture: texture_2d<f32>;
@group(0) @binding(2)
var s_texture: sampler;

struct Instance {
  @location(1) min: vec2<f32>,
  @location(2) max: vec2<f32>,
  @location(3) uv_max: vec2<f32>,
  @location(4) uv_min: vec2<f32>,
  @location(5) corner_radius: f32,
  @location(6) edge_softness: f32,
  @location(7) col: vec4<f32>,
};

struct Vertex {
  @location(0) pos: vec2<f32>,
};

struct FsIn {
  @builtin(position) pos: vec4<f32>,
  @location(0) uv: vec2<f32>,
  @location(1) col: vec4<f32>,
};

@vertex
fn vs_main(v: Vertex, inst: Instance) -> FsIn {

  let pos_half_size = (inst.max - inst.min) / 2.0;
  let pos_center = (inst.max + inst.min) / 2.0;
  let pos = v.pos * pos_half_size + pos_center;

  let uv_half_size = (inst.uv_max - inst.uv_min) / 2.0;
  let uv_center = (inst.uv_max + inst.uv_min) / 2.0;
  let uv = v.pos * uv_half_size + uv_center;

  var out: FsIn; 
  out.pos = vec4(
    2*pos.x - 1,
    2*pos.y - 1,
    0,
    1);

  out.uv = uv;

  out.col = inst.col;
  return out;
}


@fragment
fn fs_main(in: FsIn) -> @location(0) vec4<f32> {
  return vec4(in.uv, 0, 1);
}


//@group(0) @binding(0)
//var t_viewport: texture_2d<f32>;
//
//@group(0) @binding(1)
//var s_viewport: sampler;
//
//@fragment
//fn fs_main(in: FsIn) -> @location(0) vec4<f32> {
//  return textureSample(t_viewport, s_viewport, in.uv);
//}
