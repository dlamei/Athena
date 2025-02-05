struct VsIn {
  //@location(0) pos: vec2<f32>,
};

@rust(struct Globals);

@group(0) @binding(0)
var<uniform> globals: Globals;

struct FsIn {
  @builtin(position) pos: vec4<f32>,
};

const UNIT_RECT: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
  vec2(-1, -1),
  vec2(-1,  1),
  vec2( 1, -1),
  vec2( 1,  1),
);

@vertex
fn vs_main(in: VsIn) -> FsIn {
  var out: FsIn; 
  out.pos = vec4(in.pos, 0, 1);
  //out.pos = vec4(
  //  2*dst_pos.x / globals.res.x - 1,
  //  2*dst_pos.y / globals.res.y - 1,
  //  0,
  //  1);

  return out;
}


@fragment
fn fs_main(in: FsIn) -> @location(0) vec4<f32> {
  return vec4<f32>(1, 1, 1, 1);

}
