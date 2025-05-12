use std::{fmt, str::FromStr};

use crate::camera::Camera;
use crate::{AtlasSettings, WindowData, gpu};

use egui::Rect;
use egui_probe::Probe;
use egui_tiles as tiles;
use transform_gizmo as gizmo;
use web_time::{Duration};

pub fn button_probe(
    text: &'static str,
) -> impl Fn(&mut bool, &mut egui::Ui, &egui_probe::Style) -> egui::Response {
    move |value: &mut bool, ui: &mut egui::Ui, _: &egui_probe::Style| -> egui::Response {
        let resp = ui.add_enabled(!*value, egui::widgets::Button::new(text));
        if resp.clicked() {
            *value = true
        }
        resp
    }
}

pub fn f32_drag(
    speed: f32,
) -> impl Fn(&mut f32, &mut egui::Ui, &egui_probe::Style) -> egui::Response {
    move |value: &mut f32, ui: &mut egui::Ui, _: &egui_probe::Style| -> egui::Response {
        let mut v = *value;
        let mut resp = ui.add(egui::DragValue::new(&mut v).speed(speed));
        if v != *value {
            *value = v;
            // resp.changed = true;
        }
        resp
    }
}

pub fn f64_drag(
    speed: f64,
) -> impl Fn(&mut f64, &mut egui::Ui, &egui_probe::Style) -> egui::Response {
    move |value: &mut f64, ui: &mut egui::Ui, _: &egui_probe::Style| -> egui::Response {
        let mut v = *value;
        let mut resp = ui.add(egui::DragValue::new(&mut v).speed(*value / 10.0));
        if v != *value {
            *value = v;
            // resp.changed = true;
        }
        resp
    }
}

pub fn angle_probe_deg(
    value: &mut f32,
    ui: &mut egui::Ui,
    _: &egui_probe::Style,
) -> egui::Response {
    let mut degrees = *value;
    let mut resp = ui.add(egui::DragValue::new(&mut degrees).speed(1.0).suffix("°"));

    if degrees != *value {
        *value = degrees;
        // resp.changed = true;
    }

    resp
}

pub fn label_probe<T: fmt::Display>(
    value: &mut T,
    ui: &mut egui::Ui,
    _: &egui_probe::Style,
) -> egui::Response {
    ui.label(format!("{value}"))
}

pub fn duration_probe(
    value: &mut Duration,
    ui: &mut egui::Ui,
    _: &egui_probe::Style,
) -> egui::Response {
    ui.label(format!("{:0.2} μs", value.as_micros()))
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct SciFloat<F> {
    v: F,
    prec: usize,
    bounds: (f64, f64),
}

impl<F: Default> Default for SciFloat<F> {
    fn default() -> Self {
        Self {
            v: F::default(),
            prec: 3,
            bounds: (1e-3, 1e6),
        }
    }
}

impl<F: Default> SciFloat<F> {
    fn new(v: F) -> Self {
        Self {
            v,
            ..Default::default()
        }
    }
}

impl<F: Into<f64> + Copy> fmt::Display for SciFloat<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v: f64 = self.v.into();
        let abs = v.abs();

        if abs != 0.0 && (abs < self.bounds.0 || abs > self.bounds.1) {
            write!(f, "{:.*e}", self.prec, v)
        } else {
            write!(f, "{v}")
        }
    }
}

pub fn dvec2_probe(
    v: &mut glam::DVec2,
    ui: &mut egui::Ui,
    _: &egui_probe::Style,
) -> egui::Response {
    let width = ui.available_width();
    ui.columns(2, |ui| {
        let (x, y) = (v.x, v.y);
        ui[0].add(
            egui::DragValue::new(&mut v.x)
                .speed(x / 10.0)
                .custom_formatter(|a, b| SciFloat::new(a).to_string()),
        );
        ui[1].add(
            egui::DragValue::new(&mut v.y)
                .speed(y / 10.0)
                .custom_formatter(|a, b| SciFloat::new(a).to_string()),
        )
        // ui[1].add(egui::DragValue::new(&mut v.y).speed(y / 10.0))
    })
}

pub fn vec2_probe(v: &mut glam::Vec2, ui: &mut egui::Ui, _: &egui_probe::Style) -> egui::Response {
    let width = ui.available_width();
    ui.columns(2, |ui| {
        ui[0].add(egui::DragValue::new(&mut v.x));
        ui[1].add(egui::DragValue::new(&mut v.y))
    })
}

pub fn vec3_probe(v: &mut glam::Vec3, ui: &mut egui::Ui, _: &egui_probe::Style) -> egui::Response {
    let width = ui.available_width();
    ui.columns(3, |ui| {
        ui[0].add(egui::DragValue::new(&mut v.x));
        ui[1].add(egui::DragValue::new(&mut v.y));
        ui[2].add(egui::DragValue::new(&mut v.z))
    })
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum UiTab {
    Viewport,
    Settings,
    Placeholder,
}

impl fmt::Display for UiTab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UiTab::Viewport => write!(f, "viewport"),
            UiTab::Placeholder => write!(f, "placeholder"),
            UiTab::Settings => write!(f, "settings"),
        }
    }
}

pub struct UiAccess<'a> {
    // pub vp_texture: &'a gpu::Texture,
    pub vp_texture: egui::TextureId,
    pub camera: &'a Camera,
    pub window_info: &'a mut WindowData,
    //vp_dragged: &'a mut bool,
    //vp_rect: &'a mut egui::Rect,
    pub settings: &'a mut AtlasSettings,
}

//type UiDemo = egui_demo_lib::WidgetGallery;

//pub struct UiViewer<'a> {
//    pub access: UiAccess<'a>,
//    //ui_demo: &'a mut UiDemo,
//    pub egui_ctx: &'a egui::Context,
//}

pub fn selection<T>(label: &str, curr_mut: &mut T, options: &[(T, &str)], ui: &mut egui::Ui)
where
    T: PartialEq + Copy,
{
    let curr_name = options
        .iter()
        .find(|(val, _)| *val == *curr_mut)
        .expect("could not find current selection in options")
        .1;

    egui::ComboBox::from_label(label)
        .selected_text(curr_name.to_string())
        .show_ui(ui, |ui| {
            for (opt, opt_name) in options {
                ui.selectable_value(curr_mut, *opt, opt_name.to_string());
            }
        });
}

fn edit_field<T: fmt::Display + FromStr + Default + PartialEq>(
    ui: &mut egui::Ui,
    value: &mut T,
) -> egui::Response {
    let default = T::default();

    let mut tmp_val = if *value == default {
        "".to_string()
    } else {
        format!("{}", value)
    };

    let res = ui.text_edit_singleline(&mut tmp_val);
    if tmp_val.is_empty() {
        *value = Default::default();
    } else if let Ok(result) = tmp_val.parse() {
        *value = result;
    }
    res
}

impl UiAccess<'_> {
    fn viewport(&mut self, ui: &mut egui::Ui, tile_id: tiles::TileId) -> tiles::UiResponse {
        let min = ui.cursor().min;

        let uv = Rect::from_min_max([0., 0.].into(), [1., 1.].into());
        ui.painter().image(
            self.vp_texture,
            ui.max_rect(),
            uv,
            egui::Color32::WHITE,
        );

        //ui.allocate_space(ui.available_size());
        let resp = ui.allocate_rect(ui.max_rect(), egui::Sense::drag());

        self.window_info.viewport_rect = resp.rect;
        self.window_info.viewport_dragged = resp.dragged();

        // let gizmo = &mut self.gizmo;

        // let mut config = gizmo.config().clone();
        // let view = self.camera.view_mat().as_dmat4();
        // let proj = self.camera.proj_mat().as_dmat4();
        // let vp_rect = resp.rect;

        // config.view_matrix = mint::RowMatrix4::from(view);
        // config.projection_matrix = mint::RowMatrix4::from(proj);
        // config.viewport = gizmo::Rect::from_min_max(
        //     (vp_rect.min.x, vp_rect.min.y).into(),
        //     (vp_rect.max.x, vp_rect.max.y).into(),
        // );
        // config.pixels_per_point = self.window_info.ui_pixel_per_point;

        // gizmo.update_config(config);

        // let hover_pos = resp.hover_pos().unwrap_or_default();
        // let hovered = resp.hovered();

        // let gizmo_result = gizmo.update(
        //     gizmo::GizmoInteraction {
        //         cursor_pos: (hover_pos.x, hover_pos.y),
        //         hovered,
        //         drag_started: resp.drag_started(), //ui .input(|input| input.pointer.button_pressed(egui::PointerButton::Primary)),
        //         dragging: resp.dragged(), //ui.input(|input| input.pointer.button_down(egui::PointerButton::Primary)),
        //     },
        //     &[],
        // );

        // if gizmo_result.is_some() {
        //     self.window_info.viewport_dragged = false;
        // }

        // let draw_data = gizmo.draw();

        // ui.painter().add(egui::Mesh {
        //     indices: draw_data.indices,
        //     vertices: draw_data
        //         .vertices
        //         .into_iter()
        //         .zip(draw_data.colors)
        //         .map(|(pos, [r, g, b, a])| egui::epaint::Vertex {
        //             pos: pos.into(),
        //             uv: egui::Pos2::default(),
        //             color: egui::Rgba::from_rgba_premultiplied(r, g, b, a).into(),
        //         })
        //         .collect(),
        //     ..Default::default()
        // });

        tiles::UiResponse::None
    }

    fn placeholder(&mut self, ui: &mut egui::Ui, tile_id: tiles::TileId) -> tiles::UiResponse {
        let color = egui::epaint::Rgba::from_rgb(0.2, 0.0, 0.2);
        ui.painter().rect_filled(ui.max_rect(), 0.0, color);

        //self.test_ui.ui(ui);

        let dragged = ui
            .allocate_rect(ui.max_rect(), egui::Sense::click_and_drag())
            .on_hover_cursor(egui::CursorIcon::Grab)
            .dragged();
        if dragged {
            tiles::UiResponse::DragStarted
        } else {
            tiles::UiResponse::None
        }
    }

    fn settings(&mut self, ui: &mut egui::Ui, tile_id: tiles::TileId) -> tiles::UiResponse {
        let settings = &mut self.settings;

        ui.add_space(20.0);

        Probe::new(settings).show(ui);

        ui.add_space(12.0);

        ui.collapsing("debug info", |ui| {
            ui.add_enabled_ui(false, |ui| Probe::new(&mut self.window_info).show(ui));
        });

        tiles::UiResponse::None
    }
}

impl tiles::Behavior<UiTab> for UiAccess<'_> {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        tile_id: tiles::TileId,
        pane: &mut UiTab,
    ) -> tiles::UiResponse {
        match pane {
            UiTab::Viewport => self.viewport(ui, tile_id),
            UiTab::Placeholder => self.placeholder(ui, tile_id),
            UiTab::Settings => self.settings(ui, tile_id),
        }
    }

    fn tab_title_for_pane(&mut self, pane: &UiTab) -> egui::WidgetText {
        format!("{pane}").into()
    }

    fn simplification_options(&self) -> tiles::SimplificationOptions {
        tiles::SimplificationOptions {
            ..Default::default()
        }
    }
}

pub struct UiState {
    pub tile_state: tiles::Tree<UiTab>,
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

impl UiState {
    pub fn new() -> Self {
        let mut tiles = tiles::Tiles::default();

        let vp = tiles.insert_pane(UiTab::Viewport);
        let root = tiles.insert_tab_tile(vec![vp]);

        let tabs = vec![root, tiles.insert_pane(UiTab::Settings)];
        let root = tiles.insert_horizontal_tile(tabs);

        let tile_state = tiles::Tree::new("tiles", root, tiles);

        Self { tile_state }
    }

    pub fn ui(&mut self, ctx: &egui::Context, mut access: UiAccess) {
        // let ui_demo = &mut self.ui_demo;

        egui::CentralPanel::default()
            //.frame(egui::Frame::central_panel(&ctx.style()))
            .show(ctx, |ui| {
                self.tile_state.ui(&mut access, ui);
            });
    }
}
