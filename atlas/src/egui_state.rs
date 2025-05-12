use std::sync::{Arc, RwLock};

pub struct EguiState {
    pub(crate) context: egui::Context,
    pub(crate) window_state: egui_winit::State,
    pub(crate) wgpu_state: Arc<RwLock<egui_wgpu::Renderer>>,

    pub full_output: Option<egui::FullOutput>,
}

impl EguiState {
    pub fn new(
        device: &wgpu::Device,
        output_color_format: wgpu::TextureFormat,
        output_depth_format: Option<wgpu::TextureFormat>,
        msaa_samples: u32,
        window: &winit::window::Window,
    ) -> EguiState {
        let egui_context = egui::Context::default();
        let id = egui_context.viewport_id();

        // let mut fonts = egui::FontDefinitions::default();
        // fonts.font_data.insert(
        //     "CommitMono".into(),
        //     egui::FontData::from_static(include_bytes!("CommitMono-400-Regular.otf")).into(),
        // );

        // fonts.families.insert(
        //     egui::FontFamily::Name("CommitMono".into()),
        //     vec!["CommitMono".to_owned()],
        // );

        // fonts
        //     .families
        //     .get_mut(&egui::FontFamily::Proportional)
        //     .unwrap() //it works
        //     .insert(0, "CommitMono".to_owned());

        // fonts
        //     .families
        //     .get_mut(&egui::FontFamily::Monospace)
        //     .unwrap()
        //     .insert(0, "CommitMono".to_owned()); //.push("CommitMono".to_owned());

        // egui_context.set_fonts(fonts);

        //let visuals = egui::Visuals {
        //    window_rounding: egui::Rounding::same(0.0),
        //    window_shadow: egui::epaint::Shadow::NONE,
        //    ..Default::default()
        //};

        //egui_context.set_visuals(visuals);

        let mut old = egui_context.style().visuals.clone();
        old.window_stroke.width = 0.0;
        old.clip_rect_margin = 0.0;
        let dark_theme = make_visuals(&catppuccin_egui::MACCHIATO, old.clone());
        let light_theme = make_visuals(&catppuccin_egui::LATTE, old);

        egui_context.style_mut_of(egui::Theme::Dark, |style| {
            style.visuals = dark_theme;
            for (_text_style, font_id) in style.text_styles.iter_mut() {
                font_id.size = 16.0;
            }
        });
        egui_context.style_mut_of(egui::Theme::Light, |style| {
            style.visuals = light_theme;
            for (_text_style, font_id) in style.text_styles.iter_mut() {
                font_id.size = 16.0;
            }
        });

        let window_state =
            egui_winit::State::new(egui_context.clone(), id, &window, None, None, None);
        //egui_winit::State::new(egui_context.clone(), id, &window, None, None, None);

        let wgpu_state = egui_wgpu::Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
            true,
        );

        EguiState {
            context: egui_context,
            window_state,
            wgpu_state: RwLock::new(wgpu_state).into(),
            full_output: None,
        }
    }

    pub fn handle_input(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) {
        let _ = self.window_state.on_window_event(window, event);
    }

    pub fn update(
        &mut self,
        window: &winit::window::Window,
        mut ui_callback: impl FnMut(&egui::Context),
    ) {
        let raw_input = self.window_state.take_egui_input(window);
        let full_output = self.context.run(raw_input, |ctx| ui_callback(ctx));
        self.full_output = full_output.into();
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        window: &winit::window::Window,
        target: &wgpu::TextureView,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
    ) {
        let full_output = if let Some(fo) = self.full_output.take() {
            fo
        } else {
            return;
        };

        self.window_state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .context
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let mut wgpu_state = self.wgpu_state.write().unwrap();

        for (id, image_delta) in &full_output.textures_delta.set {
            wgpu_state.update_texture(device, queue, *id, image_delta);
        }
        wgpu_state.update_buffers(device, queue, encoder, &tris, &screen_descriptor);

        {
            let mut rpass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    label: Some("egui main render pass"),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();
            wgpu_state.render(&mut rpass, tris.as_slice(), &screen_descriptor);
        }

        for x in &full_output.textures_delta.free {
            wgpu_state.free_texture(x)
        }
    }
}

macro_rules! cast_col {
    ($c:expr) => {{
        let (r, g, b, a) = $c.to_tuple();
        egui::Color32::from_rgba_premultiplied(r, g, b, a)
    }};
}
fn make_widget_visual(
    old: egui::style::WidgetVisuals,
    theme: &catppuccin_egui::Theme,
    bg_fill: egui::Color32,
) -> egui::style::WidgetVisuals {
    egui::style::WidgetVisuals {
        bg_fill,
        weak_bg_fill: bg_fill,
        bg_stroke: egui::Stroke {
            color: cast_col!(theme.overlay1),
            ..old.bg_stroke
        },
        fg_stroke: egui::Stroke {
            color: cast_col!(theme.text),
            ..old.fg_stroke
        },
        ..old
    }
}

fn make_visuals(theme: &catppuccin_egui::Theme, old: egui::Visuals) -> egui::Visuals {
    let is_latte = *theme == catppuccin_egui::LATTE;
    let shadow_color = if is_latte {
        egui::Color32::from_black_alpha(25)
    } else {
        egui::Color32::from_black_alpha(96)
    };

    egui::Visuals {
        override_text_color: Some(cast_col!(theme.text)),
        hyperlink_color: cast_col!(theme.rosewater),
        faint_bg_color: cast_col!(theme.surface0),
        extreme_bg_color: cast_col!(theme.crust),
        code_bg_color: cast_col!(theme.mantle),
        warn_fg_color: cast_col!(theme.peach),
        error_fg_color: cast_col!(theme.maroon),
        window_fill: cast_col!(theme.base),
        panel_fill: cast_col!(theme.base),
        window_stroke: egui::Stroke {
            color: cast_col!(theme.overlay1),
            ..old.window_stroke
        },
        widgets: egui::style::Widgets {
            noninteractive: make_widget_visual(
                old.widgets.noninteractive,
                theme,
                cast_col!(theme.base),
            ),
            inactive: make_widget_visual(old.widgets.inactive, theme, cast_col!(theme.surface0)),
            hovered: make_widget_visual(old.widgets.hovered, theme, cast_col!(theme.surface2)),
            active: make_widget_visual(old.widgets.active, theme, cast_col!(theme.surface1)),
            open: make_widget_visual(old.widgets.open, theme, cast_col!(theme.surface0)),
        },
        selection: egui::style::Selection {
            bg_fill: cast_col!(theme.blue.linear_multiply(if is_latte { 0.4 } else { 0.2 })),
            stroke: egui::Stroke {
                color: cast_col!(theme.overlay1),
                ..old.selection.stroke
            },
        },

        window_shadow: egui::epaint::Shadow {
            color: shadow_color,
            ..old.window_shadow
        },
        popup_shadow: egui::epaint::Shadow {
            color: shadow_color,
            ..old.popup_shadow
        },
        dark_mode: !is_latte,
        ..old
    }
}
