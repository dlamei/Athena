use glam::{DVec2, DVec3, Vec3, Vec4Swizzles};
use wgpu::util::DeviceExt;

#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub pos: Vec3,
}

impl Vertex {
    pub fn cube(min: Vec3, max: Vec3) -> Vec<Self> {
        let positions = [
            // Vec3::new(0.0, 0.0, 0.0), // 0
            min,
            min.with_x(max.x),
            // Vec3::new(1.0, 0.0, 0.0), // 1
            max.with_z(min.z),
            // Vec3::new(1.0, 1.0, 0.0), // 2
            min.with_y(max.y),
            // Vec3::new(0.0, 1.0, 0.0), // 3
            min.with_z(max.z),
            // Vec3::new(0.0, 0.0, 1.0), // 4
            max.with_y(min.y),
            // Vec3::new(1.0, 0.0, 1.0), // 5
            max,
            max.with_x(min.x),
            // Vec3::new(0.0, 1.0, 1.0), // 7
        ];

        let indices: [[usize; 3]; 12] = [
            [0, 1, 2],
            [0, 2, 3], // front
            [5, 4, 7],
            [5, 7, 6], // back
            [4, 0, 3],
            [4, 3, 7], // left
            [1, 5, 6],
            [1, 6, 2], // right
            [3, 2, 6],
            [3, 6, 7], // top
            [4, 5, 1],
            [4, 1, 0], // bottom
        ];

        indices
            .iter()
            .flat_map(|&[i1, i2, i3]| {
                vec![
                    Vertex { pos: positions[i1] },
                    Vertex { pos: positions[i2] },
                    Vertex { pos: positions[i3] },
                ]
            })
            .collect()
    }
    pub fn unit_cube() -> Vec<Self> {
        let positions = [
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(1.0, 0.0, 0.0), // 1
            Vec3::new(1.0, 1.0, 0.0), // 2
            Vec3::new(0.0, 1.0, 0.0), // 3
            Vec3::new(0.0, 0.0, 1.0), // 4
            Vec3::new(1.0, 0.0, 1.0), // 5
            Vec3::new(1.0, 1.0, 1.0), // 6
            Vec3::new(0.0, 1.0, 1.0), // 7
        ];

        let indices: [[usize; 3]; 12] = [
            [0, 1, 2],
            [0, 2, 3], // front
            [5, 4, 7],
            [5, 7, 6], // back
            [4, 0, 3],
            [4, 3, 7], // left
            [1, 5, 6],
            [1, 6, 2], // right
            [3, 2, 6],
            [3, 6, 7], // top
            [4, 5, 1],
            [4, 1, 0], // bottom
        ];

        indices
            .iter()
            .flat_map(|&[i1, i2, i3]| {
                vec![
                    Vertex { pos: positions[i1] },
                    Vertex { pos: positions[i2] },
                    Vertex { pos: positions[i3] },
                ]
            })
            .collect()
    }
}

pub const SRC: &'static str = r#"

struct Vertex {
    @location(0) pos: vec3<f32>,
}

struct WorldUniform {
    light_pos: vec3<f32>,
    _pad0: f32,
    camera_pos: vec3<f32>,
    _pad1: f32,

    line_thickness_and_pad: vec4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,

}

@group(0) @binding(0)
var<uniform> world: WorldUniform;


struct FsIn {
    @builtin(position) pos: vec4<f32>,
    @location(0) col: vec4<f32>,
}


@vertex
fn vs_main(v: Vertex) -> FsIn {

    var out: FsIn;
    out.pos = world.proj * world.view * vec4(v.pos, 1.0);
    out.col = vec4(1, 1, 1, 1);

    return out;
}


@fragment
fn fs_main(in: FsIn) -> @location(0) vec4<f32> {
    return in.col;
}
"#;

pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    vertex: wgpu::Buffer,
    n_vertices: u32,
}

impl Pipeline {
    pub fn init(wgpu: &crate::WGPU) -> Self {
        let verts = Vertex::unit_cube();

        let vertex = wgpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("graph_3d_vertex_buffer"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });

        Self {
            pipeline: load_pipeline(wgpu),
            vertex,
            n_vertices: verts.len() as u32,
        }
    }

    pub fn upload_verts(&mut self, wgpu: &crate::WGPU, verts: &[crate::Vertex]) {
        let verts_3d: Vec<_> = verts
            .into_iter()
            .map(|v| Vertex { pos: v.pos.xyz() })
            .collect();
        self.n_vertices = verts.len() as u32;
        self.vertex = wgpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("graph_3d_vertex_buffer"),
                contents: bytemuck::cast_slice(&verts_3d),
                usage: wgpu::BufferUsages::VERTEX,
            });
    }

    pub fn update(&mut self, wgpu: &crate::WGPU, config: &crate::iso_3d::Iso3DConfig) {
        let verts = crate::iso_3d::build(config);
        self.n_vertices = verts.len() as u32;
        self.vertex = wgpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("graph_3d_vertex_buffer"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
    }

    pub fn render(
        &self,
        wgpu: &crate::WGPU,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        world_uniform: &wgpu::BindGroup,
        resolve: Option<&wgpu::TextureView>,
    ) {
        if self.n_vertices == 0 {
            return;
        }

        let mut encoder = wgpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render 3D graph Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: resolve,
                    ops: wgpu::Operations {
                        // load: wgpu::LoadOp::Clear(crate::hex_to_col("#1b1b1b")),
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations {
                        // load: wgpu::LoadOp::Clear(1.0),
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                label: None,
                occlusion_query_set: None,
            });

            render_pass.set_vertex_buffer(0, self.vertex.slice(..));
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, world_uniform, &[]);
            render_pass.draw(0..self.n_vertices, 0..1);
        }
        wgpu.queue.submit(std::iter::once(encoder.finish()));
    }
}

pub fn load_pipeline(wgpu: &crate::WGPU) -> wgpu::RenderPipeline {
    let world_bind_group_layout = crate::WorldUniform::layout(wgpu);

    let shader_module = wgpu
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("graph_3d"),
            source: wgpu::ShaderSource::Wgsl(SRC.into()),
        });

    let pipeline_layout = wgpu
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("graph_3d_pipeline_layout"),
            bind_group_layouts: &[&world_bind_group_layout],
            push_constant_ranges: &[],
        });

    let pipeline = wgpu
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("graph_3d_line_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu.surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                unclipped_depth: false,
                front_face: wgpu::FrontFace::Ccw,
                polygon_mode: wgpu::PolygonMode::Line,
                strip_index_format: None,
                topology: wgpu::PrimitiveTopology::TriangleList,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::AtlasRenderer::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: crate::multisample_state(),
            multiview: None,
            cache: None,
        });

    pipeline
}
