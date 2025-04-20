@rust(struct WorldUniform);
@rust(struct VertexInput)

@group(0) @binding(0)
var<uniform> world: WorldUniform;

struct FsIn {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) col: vec4<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> FsIn {
    var out: FsIn;

    out.col = model.col;
    out.clip_pos = world.view_proj * vec4<f32>(model.pos.xyz, 1.0);
    out.clip_pos = world.view_proj * vec4<f32>(model.pos.xyz, 1.0);
    return out;
}

@fragment
fn fs_main(in: FsIn) -> @location(0) vec4<f32> {
  //return vec4(in.col * 0.5 + 0.5, 1.0);
  return vec4(in.col);
}
