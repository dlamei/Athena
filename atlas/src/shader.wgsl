struct WorldUniform {
    light_pos: vec3<f32>,
    _pad0: f32,
    camera_pos: vec3<f32>,
    _pad1: f32,

    line_thickness_and_pad: vec4<f32>,
    //view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,

}

@group(0) @binding(0)
var<uniform> world: WorldUniform;

struct FsIn {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) col: vec4<f32>,
};

struct VertexInput {
  @location(0) pos: vec4<f32>,
  @location(1) col: vec4<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> FsIn {
    var out: FsIn;

    out.col = model.col;
    out.clip_pos = world.proj * world.view * vec4<f32>(model.pos.xyz, 1.0);
    return out;
}

@fragment
fn fs_main(in: FsIn) -> @location(0) vec4<f32> {
  //return vec4(in.col * 0.5 + 0.5, 1.0);
  return vec4(in.col);
}
