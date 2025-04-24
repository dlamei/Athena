use glam::{Vec2, Vec4};
use macros::ShaderStruct;

use crate::gpu;

#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub pos: Vec2,
}

#[derive(Default, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Instance {
    pub min: Vec2,
    pub max: Vec2,
    pub uv_min: Vec2,
    pub uv_max: Vec2,
    pub col: Vec4,
}

impl gpu::VertexDescription for Instance {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![1 => Float32x2, 2 => Float32x2, 3 => Float32x2, 4 => Float32x2, 5 => Float32x4];
}

#[derive(Default, Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, ShaderStruct)]
#[repr(C)]
pub struct Globals {
    pub res: Vec2,
}

impl gpu::VertexDescription for Vertex {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![0 => Float32x2];
    // const ATTRIBUTES: &'static [wgpu::VertexAttribute] =
    //     &wgpu::vertex_attr_array![1 => Float32x2, 2 => Float32x2, 3 => Float32x4, 4 => Uint32, 5 => Float32x3];
}

// inspired by egui::emath::Rect
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub const fn from_min_max(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub const fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub const fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    pub const fn top(&self) -> f32 {
        self.max.y
    }

    pub const fn bot(&self) -> f32 {
        self.min.y
    }

    pub const fn left(&self) -> f32 {
        self.min.x
    }

    pub const fn right(&self) -> f32 {
        self.max.x
    }

    pub const fn top_left(&self) -> Vec2 {
        Vec2::new(self.left(), self.top())
    }

    pub const fn top_right(&self) -> Vec2 {
        Vec2::new(self.right(), self.top())
    }

    pub const fn bot_left(&self) -> Vec2 {
        Vec2::new(self.left(), self.bot())
    }

    pub const fn bot_right(&self) -> Vec2 {
        Vec2::new(self.right(), self.bot())
    }

    pub const fn mid_x(&self) -> f32 {
        (self.left() + self.right()) / 2.0
    }

    pub const fn mid_y(&self) -> f32 {
        (self.bot() + self.top()) / 2.0
    }

    pub const fn mid(&self) -> Vec2 {
        Vec2::new(self.mid_x(), self.mid_y())
    }

    //pub const fn as_verts(&self) -> [Vertex; 6] {
    //    [
    //        Vertex {
    //            pos: self.bot_left(),
    //            uv: Vec2::ZERO,
    //        },
    //        Vertex {
    //            pos: self.bot_right(),
    //            uv: Vec2::X,
    //        },
    //        Vertex {
    //            pos: self.top_right(),
    //            uv: Vec2::ONE,
    //        },
    //        Vertex {
    //            pos: self.bot_left(),
    //            uv: Vec2::ZERO,
    //        },
    //        Vertex {
    //            pos: self.top_right(),
    //            uv: Vec2::ONE,
    //        },
    //        Vertex {
    //            pos: self.top_left(),
    //            uv: Vec2::Y,
    //        },
    //    ]
    //}
}

/*
struct Quad {
    rect: Rect,
    color: Vec4,
}

impl Quad {
    pub const fn as_verts(&self, col: Vec4) -> ([Vertex; 4], [u32; 6]) {
        ([
            Vertex {
                pos: self.rect.bot_left(),
                uv: Vec2::ZERO,
                col,
            },
            Vertex {
                pos: self.rect.bot_right(),
                uv: Vec2::X,
                col,
            },
            Vertex {
                pos: self.rect.top_right(),
                uv: Vec2::ONE,
                col,
            },
            Vertex {
                pos: self.rect.top_left(),
                uv: Vec2::Y,
                col,
            },
        ], [0, 1, 2, 2, 3, 0])
    }
}
*/

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
enum UiSizeKind {
    #[default]
    None,
    Pixels,
    PercentOfParent,
    ChildrenSum,
}

// https://www.rfleury.com/p/ui-part-2-build-it-every-frame-immediate
#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct UiSize {
    kind: UiSizeKind,
    value: f32,
    strictness: f32,
}

mod WidgetFlag {
    pub const CLICKABLE: u32 = 1 << 0;
    pub const VIEW_SCROLL: u32 = 1 << 1;
    pub const DRAW_TEXT: u32 = 1 << 2;
    pub const DRAW_BORDER: u32 = 1 << 3;
    pub const DRAW_BACKGROUND: u32 = 1 << 4;
    pub const DRAW_DROP_SHADOW: u32 = 1 << 5;
    pub const CLIP: u32 = 1 << 6;
    pub const HOT_ANIMATION: u32 = 1 << 7;
    pub const ACTIVE_ANIMATION: u32 = 1 << 8;
}

type WidgetFlags = u32;

#[derive(Debug, Default, Clone, PartialEq)]
struct Widget {
    // per-frame provided by builder
    flags: WidgetFlags,
    string: Option<String>,
    semantic_size: (UiSize, UiSize),

    first: UIID,
    last: UIID,
    next: UIID,
    prev: UIID,
    parent: UIID,

    // hash_next: UIID,
    // hash_prev: UIID,

    // key: UiKey,
    // last_frame_touched_indx: u64,

    // compute every frame
    computed_rel_pos: (f32, f32),
    computed_size: (f32, f32),
    rect: Rect,

    // persistent data
    hot: f32,
    active: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct UiResponse {
    widget: UIID,
    mouse: Vec2,
    drag_delta: Vec2,
    clicked: bool,
    double_clicked: bool,
    right_clicked: bool,
    pressed: bool,
    released: bool,
    dragging: bool,
    hovering: bool,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UIID(usize);

impl Default for UIID {
    fn default() -> Self {
        Self::NULL
    }
}

impl UIID {
    pub const NULL: Self = Self(usize::MAX);
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UiKey(usize);

impl Default for UiKey {
    fn default() -> Self {
        Self::NULL
    }
}

impl UiKey {
    pub const NULL: Self = Self(usize::MAX);
}

#[derive(Debug, Default)]
pub struct UiContext {
    widgets: Vec<Widget>,
}

impl UiContext {
    pub fn make_widget(flags: WidgetFlags) -> UIID {
        let widget = Widget {
            flags,
            ..Default::default()
        };
        todo!()
    }

    pub fn make_widget_str(flags: WidgetFlags, str: String) -> UIID {
        let widget = Widget {
            flags,
            ..Default::default()
        };
        todo!()
    }
}
