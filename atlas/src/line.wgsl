//struct Globals {
//  thickness: f32,
//};

@group(0) @binding(0)
var<uniform> world: WorldUniform;


struct VertexInput {
  @location(0) pos: vec4<f32>,
  @location(1) col: vec4<f32>,
}

struct Instance {
  @location(2) a: vec3<f32>,
  @location(3) b: vec3<f32>,
};

struct WorldUniform {
  light_pos: vec3<f32>,
  line_thickness: f32,
  view_proj: mat4x4<f32>,
}


//struct Vertex {
//  @location(0) pos: vec2<f32>,
//};

struct FsIn {
  @builtin(position) pos: vec4<f32>,
  @location(0) col: vec4<f32>,
};

@vertex
fn vs_main(v: VertexInput, inst: Instance, @builtin(vertex_index) indx: u32) -> FsIn {

  let a = inst.a.xyz;
  let b = inst.b.xyz;

  let dir = b - a;
  let w = normalize(cross(dir, vec3(0f, 0f, 1f))) * world.line_thickness;

  let v1 = a - w;
  let v2 = a + w;
  let v3 = b - w;
  let v4 = b + w;

  //let v1 = vec3(0f, 0f, 0f);
  //let v2 = vec3(0f, 1f, 0f);
  //let v3 = vec3(1f, 0f, 0f);
  //let v4 = vec3(1f, 1f, 0f);

  var pos: vec3<f32>;
  switch indx {
    case 0u: {
      //pos = vec3(0f, 0f, 0f);
      pos = v1;
    }
    case 1u: {
      //pos = vec3(1f, 0f, 0f);
      pos = v2;
    }
    case 2u: {
      //pos = vec3(0f, 1f, 0f);
      pos = v3;
    }
    case 3u: {
      //pos = vec3(1f, 1f, 0f);
      pos = v4;
    }
    default: {
      pos = vec3(100000f);
    }
  }

  var out: FsIn;
  out.pos = world.view_proj * vec4(pos, 1.0); //vec4(pos, 1.0);
  out.col = v.col;
  return out;
}


@fragment
fn fs_main(in: FsIn) -> @location(0) vec4<f32> {
  return in.col;
  //return vec4(1f, 1f, 1f, 1f);
  //return vec4(in.uv, 0, 1);
}
