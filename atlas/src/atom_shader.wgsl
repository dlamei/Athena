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

struct VertexInput {
  @location(0) pos: vec3<f32>,
  @location(1) norm: vec3<f32>,
}

struct VertexInstance {
  pos: vec3<f32>,
  label_id: u32,
};


@group(0) @binding(0)
var<uniform> world: WorldUniform;

@group(1) @binding(0)
var<storage> atoms_data: array<VertexInstance>;

@group(1) @binding(1)
var<storage> mol_symmetries: array<mat4x4<f32>>;


const SYMBOL_N: u32 = 110;
const SYMBOL_C: u32 = 99;
const SYMBOL_O: u32 = 111;

struct FsIn {
  @builtin(position) pos: vec4<f32>,
  @location(0) col: vec4<f32>,
  @location(1) norm: vec3<f32>,
  @location(2) world_pos: vec3<f32>,
}

@vertex
fn vs_main(v: VertexInput, @builtin(instance_index) instance_id: u32) -> FsIn {

  let n_base_atoms = arrayLength(&atoms_data);
  let base_indx = instance_id % n_base_atoms;
  let symmetry_indx = instance_id / n_base_atoms;

  let atom_data = atoms_data[base_indx];
  let symmetry = mol_symmetries[symmetry_indx];

  let world_pos = (symmetry * vec4(atom_data.pos, 1.0)).xyz + v.pos;

  var out: FsIn;

  switch atom_data.label_id {
    case 1u: { out.col = vec4(0.78, 0.12, 0.28, 1.0); }
    case 2u: { out.col = vec4(0.1, 0.62, 0.46, 1.0); }
    case 3u: { out.col = vec4(0.3, 0.38, 0.84, 1.0); }
    case 4u: { out.col = vec4(0.65, 0.65, 0.7, 1.0); }
    default: { out.col = vec4(0.65, 0.65, 0.7, 1.0); }
  }

  //let world_pos = v.pos;
  out.world_pos = (world.view * vec4(world_pos, 1.0)).xyz;

  // out.world_pos = world_pos;
  out.norm = normalize(v.norm);
  out.pos = world.proj * world.view * vec4(world_pos, 1.0);

  return out;
}

@fragment
fn fs_main(in: FsIn) -> @location(0) vec4<f32> {
    let N = normalize(in.norm);
    let L = normalize(world.camera_pos);

    let ambient = 0.1;
    let diff    = max(dot(N, L), 0.0);

    let lighting = ambient + diff; // + spec * 0.3;

    let rgb = in.col.rgb * lighting;
    return vec4(rgb, in.col.a);
}

