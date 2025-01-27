@rust(struct WorldUniform);
@rust(struct VertexInput)

@group(0) @binding(0)
var<uniform> world: WorldUniform;

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) norm: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.norm = model.norm;
    out.clip_pos = world.view_proj * vec4<f32>(model.pos, 1.0);
    out.world_pos = model.pos;
    return out;
}

const thickness: f32 = 0.001;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  //let dir_light = normalize(world.light_pos - in.world_pos);
  //let light = dot(in.norm, dir_light);
  return vec4(in.norm * 0.5 + 0.5, 1.0);
  //return vec4(vec3(light), 1.0);
  //return vec4<f32>(1.0, 1.0, 1.0, 0.005);
}
