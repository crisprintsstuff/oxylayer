use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use oxylayer_brush::{BrushEngine, BrushSettings};
use oxylayer_core::{
    BlendMode, Document, HistoryManager, Layer, PixelBuffer, Rgba8,
};
use oxylayer_filter::{gaussian_blur, vignette, ColorAdjustments};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTool {
    Select,
    Wand,
    ColorGrading,
    Effects,
    Marquee,
    Retouch,
    Brush,
    Eraser,
    ColorPicker,
    Pen,
    Text,
    Crop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    Effects,
    ColorAdjustments,
}

pub struct OxyLayerApp {
    document: Document,
    history: HistoryManager,
    active_tool: ActiveTool,
    active_sidebar_tab: SidebarTab,
    brush_settings: BrushSettings,
    color_adjustments: ColorAdjustments,

    // Panels visibility
    show_layers_panel: bool,
    show_effects_panel: bool,

    // Vignette effect params
    vignette_enabled: bool,
    vignette_radius: f32,
    vignette_intensity: f32,
    vignette_falloff: f32,

    // Focus/Blur effect params
    focus_enabled: bool,
    focus_blur: f32,
    focus_transition: f32,

    // Viewport pan & zoom
    zoom: f32,
    pan: Vec2,
    last_mouse_pos: Option<glam::Vec2>,
    brush_residual_dist: f32,

    // Render cache
    canvas_texture: Option<egui::TextureHandle>,
    preset_texture: Option<egui::TextureHandle>,
    needs_recomposite: bool,

    // Status
    status_message: String,
}

impl OxyLayerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let width = 900;
        let height = 900;
        let mut document = Document::new("Viewfinder", width, height);

        // Populate realistic sample layers to match the Pixelmator Pro screenshot
        Self::setup_sample_artwork(&mut document);

        let history = HistoryManager::new(50);

        Self {
            document,
            history,
            active_tool: ActiveTool::Effects,
            active_sidebar_tab: SidebarTab::Effects,
            brush_settings: BrushSettings {
                size: 28.0,
                hardness: 0.8,
                opacity: 1.0,
                flow: 1.0,
                spacing: 0.15,
                color: Rgba8::new(255, 59, 92, 255), // Signature Pixelmator Coral
                is_eraser: false,
            },
            color_adjustments: ColorAdjustments::default(),

            show_layers_panel: true,
            show_effects_panel: true,

            vignette_enabled: true,
            vignette_radius: 0.70,
            vignette_intensity: 0.50,
            vignette_falloff: 0.30,

            focus_enabled: true,
            focus_blur: 4.1,
            focus_transition: 0.80,

            zoom: 0.85,
            pan: Vec2::ZERO,
            last_mouse_pos: None,
            brush_residual_dist: 0.0,

            canvas_texture: None,
            preset_texture: None,
            needs_recomposite: true,

            status_message: "Ready".to_string(),
        }
    }

    /// Sets up multi-layer composition with stylized viewfinder artwork
    fn setup_sample_artwork(doc: &mut Document) {
        doc.layers.clear();
        let w = doc.width;
        let h = doc.height;

        // 1. Background layer: warm cinematic tone gradient
        let mut bg_buf = PixelBuffer::new(w, h);
        for y in 0..h {
            let ty = y as f32 / h as f32;
            for x in 0..w {
                let tx = x as f32 / w as f32;
                let r = (45.0 + tx * 35.0 + ty * 20.0).clamp(0.0, 255.0) as u8;
                let g = (25.0 + tx * 15.0 + ty * 10.0).clamp(0.0, 255.0) as u8;
                let b = (35.0 + (1.0 - tx) * 20.0).clamp(0.0, 255.0) as u8;
                bg_buf.set_pixel(x, y, Rgba8::new(r, g, b, 255));
            }
        }
        let bg_layer = Layer::new_raster("Background", bg_buf);
        doc.layers.push(bg_layer);

        // 2. Center Subject Artwork: Warm decorative pattern
        let mut subject_buf = PixelBuffer::new(w, h);
        let center_x = w as f32 * 0.5;
        let center_y = h as f32 * 0.5;
        for y in 0..h {
            let dy = y as f32 - center_y;
            for x in 0..w {
                let dx = x as f32 - center_x;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < 320.0 {
                    let angle = dy.atan2(dx);
                    let pattern = (angle * 8.0).sin().abs();
                    let r = (210.0 + pattern * 45.0).clamp(0.0, 255.0) as u8;
                    let g = (45.0 + (1.0 - pattern) * 60.0).clamp(0.0, 255.0) as u8;
                    let b = (105.0 + pattern * 30.0).clamp(0.0, 255.0) as u8;
                    let falloff = ((320.0 - dist) / 60.0).clamp(0.0, 1.0);
                    subject_buf.set_pixel(x, y, Rgba8::new(r, g, b, (falloff * 255.0) as u8));
                }
            }
        }
        let mut subj_layer = Layer::new_raster("Media Placeholder: Replace", subject_buf);
        subj_layer.opacity = 0.95;
        doc.layers.push(subj_layer);

        // 3. Viewfinder Reticle Layer
        let mut frame_buf = PixelBuffer::new(w, h);
        // Draw viewfinder thin lines and corner brackets
        let bracket_color = Rgba8::new(240, 240, 245, 180);
        let inset = 70;
        let arm_len = 50;

        // Top-left bracket
        for x in inset..inset + arm_len {
            frame_buf.set_pixel(x, inset, bracket_color);
            frame_buf.set_pixel(x, inset + 1, bracket_color);
        }
        for y in inset..inset + arm_len {
            frame_buf.set_pixel(inset, y, bracket_color);
            frame_buf.set_pixel(inset + 1, y, bracket_color);
        }

        // Top-right bracket
        for x in (w - inset - arm_len)..(w - inset) {
            frame_buf.set_pixel(x, inset, bracket_color);
            frame_buf.set_pixel(x, inset + 1, bracket_color);
        }
        for y in inset..inset + arm_len {
            frame_buf.set_pixel(w - inset - 1, y, bracket_color);
            frame_buf.set_pixel(w - inset - 2, y, bracket_color);
        }

        // Bottom-left bracket
        for x in inset..inset + arm_len {
            frame_buf.set_pixel(x, h - inset - 1, bracket_color);
            frame_buf.set_pixel(x, h - inset - 2, bracket_color);
        }
        for y in (h - inset - arm_len)..(h - inset) {
            frame_buf.set_pixel(inset, y, bracket_color);
            frame_buf.set_pixel(inset + 1, y, bracket_color);
        }

        // Bottom-right bracket
        for x in (w - inset - arm_len)..(w - inset) {
            frame_buf.set_pixel(x, h - inset - 1, bracket_color);
            frame_buf.set_pixel(x, h - inset - 2, bracket_color);
        }
        for y in (h - inset - arm_len)..(h - inset) {
            frame_buf.set_pixel(w - inset - 1, y, bracket_color);
            frame_buf.set_pixel(w - inset - 2, y, bracket_color);
        }

        let frame_layer = Layer::new_raster("Frame Effect", frame_buf);
        doc.layers.push(frame_layer);

        // 4. Focusing Screen Ring
        let mut focus_buf = PixelBuffer::new(w, h);
        let focus_color = Rgba8::new(255, 255, 255, 140);
        let ring_radius = 65.0;
        for y in 0..h {
            let dy = y as f32 - center_y;
            for x in 0..w {
                let dx = x as f32 - center_x;
                let dist = (dx * dx + dy * dy).sqrt();
                if (dist - ring_radius).abs() <= 1.0 {
                    focus_buf.set_pixel(x, y, focus_color);
                }
            }
        }
        let focus_layer = Layer::new_raster("Focusing Screen Ring", focus_buf);
        doc.layers.push(focus_layer);

        doc.selected_layer_id = doc.layers.last().map(|l| l.id);
    }

    /// Rerenders the document layer composite into an egui TextureHandle
    fn update_texture(&mut self, ctx: &egui::Context) {
        let mut composite = self.document.composite();

        // Apply Vignette if enabled
        if self.vignette_enabled && self.vignette_intensity > 0.01 {
            vignette(&mut composite, self.vignette_radius, self.vignette_falloff);
        }

        // Apply Blur/Focus if enabled
        if self.focus_enabled && self.focus_blur > 0.1 {
            let blurred = gaussian_blur(&composite, self.focus_blur);
            // Blend focus center with blurred background
            let w = composite.width();
            let h = composite.height();
            let cx = w as f32 * 0.5;
            let cy = h as f32 * 0.5;
            let clear_radius = 160.0 * self.focus_transition;

            for y in 0..h {
                let dy = y as f32 - cy;
                for x in 0..w {
                    let dx = x as f32 - cx;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let blur_weight = ((dist - clear_radius) / 80.0).clamp(0.0, 1.0);
                    if blur_weight > 0.0 {
                        let orig = composite.get_pixel(x, y).unwrap().to_f32();
                        let blur_px = blurred.get_pixel(x, y).unwrap().to_f32();
                        let blended = BlendMode::Normal.blend(orig, blur_px, blur_weight);
                        composite.set_pixel(x, y, blended.to_rgba8());
                    }
                }
            }
        }

        // Apply Color Adjustments
        if !self.color_adjustments.is_identity() {
            self.color_adjustments.apply_to_buffer(&mut composite);
        }

        let w = composite.width() as usize;
        let h = composite.height() as usize;
        let raw = composite.as_raw();
        let image = egui::ColorImage::from_rgba_unmultiplied([w, h], raw);

        if let Some(texture) = &mut self.canvas_texture {
            texture.set(image, egui::TextureOptions::LINEAR);
        } else {
            self.canvas_texture = Some(ctx.load_texture(
                "canvas_composite",
                image,
                egui::TextureOptions::LINEAR,
            ));
        }

        // Preset thumbnail texture (aesthetic desert dunes simulation)
        if self.preset_texture.is_none() {
            let pw = 64;
            let ph = 40;
            let mut p_raw = vec![0u8; pw * ph * 4];
            for y in 0..ph {
                let ty = y as f32 / ph as f32;
                for x in 0..pw {
                    let tx = x as f32 / pw as f32;
                    let idx = (y * pw + x) * 4;
                    let wave = (tx * 4.0 + ty * 2.0).sin() * 0.2;
                    let r = ((0.85 + wave) * 255.0).clamp(0.0, 255.0) as u8;
                    let g = ((0.52 + wave * 0.5) * 255.0).clamp(0.0, 255.0) as u8;
                    let b = (0.28 * 255.0) as u8;
                    p_raw[idx] = r;
                    p_raw[idx + 1] = g;
                    p_raw[idx + 2] = b;
                    p_raw[idx + 3] = 255;
                }
            }
            let p_img = egui::ColorImage::from_rgba_unmultiplied([pw, ph], &p_raw);
            self.preset_texture = Some(ctx.load_texture("preset_thumb", p_img, egui::TextureOptions::LINEAR));
        }

        self.needs_recomposite = false;
    }

    /// Renders an Apple/Pixelmator Pro pill toggle switch
    fn draw_toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> bool {
        let desired_size = Vec2::new(38.0, 22.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());
        let mut changed = false;

        if response.clicked() {
            *on = !*on;
            changed = true;
        }

        let painter = ui.painter();
        let bg_color = if *on {
            Color32::from_rgb(255, 59, 92) // Signature Coral Pink/Red
        } else {
            Color32::from_rgb(60, 60, 65)
        };

        painter.rect_filled(rect, CornerRadius::same(11), bg_color);

        // Thumb circle
        let thumb_radius = 8.5;
        let thumb_x = if *on {
            rect.max.x - 3.0 - thumb_radius
        } else {
            rect.min.x + 3.0 + thumb_radius
        };
        let thumb_center = Pos2::new(thumb_x, rect.center().y);
        painter.circle_filled(thumb_center, thumb_radius, Color32::WHITE);

        changed
    }
}

impl eframe::App for OxyLayerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // 1. Sleek Cupertino / Pixelmator Pro Dark Theme styling
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(20, 20, 23);
        visuals.window_fill = Color32::from_rgba_unmultiplied(28, 29, 34, 240);
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(38, 39, 44);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(46, 47, 53);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(64, 66, 75);
        visuals.widgets.active.bg_fill = Color32::from_rgb(255, 59, 92);
        visuals.selection.bg_fill = Color32::from_rgb(255, 59, 92);
        ctx.set_visuals(visuals);

        // 2. Seamless Top Bar (Pills, Title, Navigation, Undo/Redo/Share)
        egui::Panel::top("top_navigation_bar")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(22, 23, 26))
                    .inner_margin(egui::Margin::symmetric(14, 10)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Left items: pill '...', '<' back, coral document icon, '+', title 'Viewfinder ⌄'
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

                    // Pill button '...'
                    if ui.add(egui::Button::new(egui::RichText::new("•••").size(12.0).strong()).corner_radius(CornerRadius::same(12))).clicked() {
                        self.show_layers_panel = !self.show_layers_panel;
                    }

                    // Back arrow '<'
                    if ui.add(egui::Button::new(egui::RichText::new("〈").size(14.0).strong()).corner_radius(CornerRadius::same(12))).clicked() {
                        self.status_message = "Gallery / Projects view".to_string();
                    }

                    ui.add_space(4.0);

                    // Coral Document Icon pill
                    let (doc_icon_rect, _resp) = ui.allocate_exact_size(Vec2::new(32.0, 26.0), Sense::hover());
                    ui.painter().rect_filled(
                        doc_icon_rect,
                        CornerRadius::same(8),
                        Color32::from_rgb(255, 75, 105), // Signature Pixelmator Coral Icon
                    );
                    ui.painter().text(
                        doc_icon_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "⧉",
                        egui::FontId::proportional(16.0),
                        Color32::WHITE,
                    );

                    // Add '+' button
                    if ui.add(egui::Button::new(egui::RichText::new("+").size(16.0)).corner_radius(CornerRadius::same(12))).clicked() {
                        self.history.push_snapshot(&self.document);
                        let w = self.document.width;
                        let h = self.document.height;
                        let count = self.document.layer_count() + 1;
                        let layer = Layer::new_raster(format!("Layer {}", count), PixelBuffer::new(w, h));
                        self.document.add_layer(layer);
                        self.needs_recomposite = true;
                    }

                    ui.add_space(4.0);

                    // Document Title with chevron
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&self.document.title)
                                .size(15.0)
                                .strong()
                                .color(Color32::from_rgb(240, 240, 245)),
                        );
                        ui.label(
                            egui::RichText::new("⌄")
                                .size(13.0)
                                .color(Color32::from_rgb(160, 160, 170)),
                        );
                    });

                    // Right items: Undo, Redo, Share/Export, '...'
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(10.0, 0.0);

                        // Document options '...'
                        let _ = ui.button(egui::RichText::new("•••").size(12.0));

                        // Share / Export icon
                        if ui.button(egui::RichText::new("📤").size(14.0)).clicked() {
                            let composite = self.document.composite();
                            let path = PathBuf::from("export.png");
                            let _ = oxylayer_io::ImageCodec::save_png(&composite, &path);
                            self.status_message = "Exported to export.png".to_string();
                        }

                        // Redo
                        let can_redo = self.history.can_redo();
                        if ui.add_enabled(can_redo, egui::Button::new(egui::RichText::new("↷").size(16.0))).clicked() {
                            if self.history.redo(&mut self.document) {
                                self.needs_recomposite = true;
                            }
                        }

                        // Undo
                        let can_undo = self.history.can_undo();
                        if ui.add_enabled(can_undo, egui::Button::new(egui::RichText::new("↶").size(16.0))).clicked() {
                            if self.history.undo(&mut self.document) {
                                self.needs_recomposite = true;
                            }
                        }
                    });
                });
            });

        // 3. Central Canvas Viewport Area
        let central_panel_response = egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::from_rgb(12, 12, 14)))
            .show(ui, |ui| {
                let (viewport_rect, response) =
                    ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());

                // Pan & Zoom with scroll wheel / middle click / space drag
                let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll_delta != 0.0 && viewport_rect.contains(response.hover_pos().unwrap_or_default()) {
                    let zoom_factor = if scroll_delta > 0.0 { 1.08 } else { 0.92 };
                    self.zoom = (self.zoom * zoom_factor).clamp(0.1, 15.0);
                }

                if response.dragged_by(egui::PointerButton::Middle)
                    || (response.dragged() && ui.input(|i| i.key_down(egui::Key::Space)))
                {
                    self.pan += response.drag_delta();
                }

                let doc_w = self.document.width as f32;
                let doc_h = self.document.height as f32;
                let scaled_w = doc_w * self.zoom;
                let scaled_h = doc_h * self.zoom;

                let canvas_center = viewport_rect.center() + self.pan;
                let canvas_rect = Rect::from_center_size(canvas_center, Vec2::new(scaled_w, scaled_h));

                // Checkerboard pattern
                let painter = ui.painter_at(viewport_rect);
                let checker_size = 14.0;
                let clip_rect = canvas_rect.intersect(viewport_rect);

                if clip_rect.is_positive() {
                    let min_x = ((clip_rect.min.x - canvas_rect.min.x) / checker_size).floor() as i32;
                    let max_x = ((clip_rect.max.x - canvas_rect.min.x) / checker_size).ceil() as i32;
                    let min_y = ((clip_rect.min.y - canvas_rect.min.y) / checker_size).floor() as i32;
                    let max_y = ((clip_rect.max.y - canvas_rect.min.y) / checker_size).ceil() as i32;

                    for y in min_y..max_y {
                        for x in min_x..max_x {
                            let is_white = (x + y) % 2 == 0;
                            let color = if is_white {
                                Color32::from_rgb(25, 25, 28)
                            } else {
                                Color32::from_rgb(18, 18, 20)
                            };
                            let tile_rect = Rect::from_min_size(
                                Pos2::new(
                                    canvas_rect.min.x + x as f32 * checker_size,
                                    canvas_rect.min.y + y as f32 * checker_size,
                                ),
                                Vec2::splat(checker_size),
                            );
                            painter.rect_filled(tile_rect.intersect(clip_rect), CornerRadius::ZERO, color);
                        }
                    }
                }

                // Recomposite document if needed
                if self.needs_recomposite || self.canvas_texture.is_none() {
                    self.update_texture(&ctx);
                }

                // Draw Document Image with rounded corners and subtle shadow
                if let Some(texture) = &self.canvas_texture {
                    painter.image(
                        texture.id(),
                        canvas_rect,
                        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }

                // Canvas outer rounded stroke frame (Pixelmator Pro viewfinder window)
                painter.rect_stroke(
                    canvas_rect,
                    CornerRadius::same(24),
                    Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 30)),
                    StrokeKind::Inside,
                );

                // Drawing with Brush / Eraser on canvas
                let pointer_pos = response.hover_pos();
                let is_drawing = response.dragged_by(egui::PointerButton::Primary)
                    && !ui.input(|i| i.key_down(egui::Key::Space));

                if is_drawing {
                    if let Some(pos) = pointer_pos {
                        if canvas_rect.contains(pos) {
                            let norm_x = (pos.x - canvas_rect.min.x) / canvas_rect.width();
                            let norm_y = (pos.y - canvas_rect.min.y) / canvas_rect.height();
                            let canvas_pt = glam::Vec2::new(norm_x * doc_w, norm_y * doc_h);

                            match self.active_tool {
                                ActiveTool::Brush | ActiveTool::Eraser => {
                                    if let Some(last_pt) = self.last_mouse_pos {
                                        if let Some(layer) = self.document.selected_layer_mut() {
                                            if let Some(buf) = layer.as_raster_mut() {
                                                BrushEngine::stroke_segment(
                                                    buf,
                                                    last_pt,
                                                    canvas_pt,
                                                    &self.brush_settings,
                                                    &mut self.brush_residual_dist,
                                                );
                                                self.needs_recomposite = true;
                                            }
                                        }
                                    } else {
                                        self.history.push_snapshot(&self.document);
                                        if let Some(layer) = self.document.selected_layer_mut() {
                                            if let Some(buf) = layer.as_raster_mut() {
                                                BrushEngine::stamp(
                                                    buf,
                                                    canvas_pt,
                                                    &self.brush_settings,
                                                );
                                                self.needs_recomposite = true;
                                            }
                                        }
                                    }
                                    self.last_mouse_pos = Some(canvas_pt);
                                }
                                _ => {}
                            }
                        }
                    }
                } else {
                    self.last_mouse_pos = None;
                    self.brush_residual_dist = 0.0;
                }

                viewport_rect
            });

        let total_rect = central_panel_response.inner;

        // 4. FLOATING FAR-RIGHT TOOL STRIP (The signature dark vertical pill bar)
        let tool_strip_width = 46.0;
        let tool_strip_margin = 14.0;
        let tool_strip_rect = Rect::from_min_size(
            Pos2::new(
                total_rect.max.x - tool_strip_width - tool_strip_margin,
                total_rect.min.y + 16.0,
            ),
            Vec2::new(tool_strip_width, 490.0),
        );

        let mut tool_ui = ui.new_child(egui::UiBuilder::new().max_rect(tool_strip_rect));
        egui::Frame::new()
            .fill(Color32::from_rgba_unmultiplied(26, 27, 31, 235))
            .corner_radius(CornerRadius::same(23))
            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 18)))
            .inner_margin(egui::Margin::symmetric(4, 8))
            .show(&mut tool_ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 5.0);

                    let mut tool_icon = |tool: ActiveTool, icon: &str| {
                        let is_active = self.active_tool == tool;
                        let text = egui::RichText::new(icon)
                            .size(15.0)
                            .color(if is_active { Color32::BLACK } else { Color32::from_rgb(200, 200, 210) });

                        let btn_bg = if is_active {
                            Color32::WHITE // Active tool glowing white pill, just like screenshot!
                        } else {
                            Color32::TRANSPARENT
                        };

                        let btn = egui::Button::new(text)
                            .fill(btn_bg)
                            .corner_radius(CornerRadius::same(17))
                            .min_size(Vec2::new(34.0, 32.0));

                        if ui.add(btn).clicked() {
                            self.active_tool = tool;
                            match tool {
                                ActiveTool::Effects => {
                                    self.show_effects_panel = true;
                                    self.active_sidebar_tab = SidebarTab::Effects;
                                }
                                ActiveTool::ColorGrading => {
                                    self.show_effects_panel = true;
                                    self.active_sidebar_tab = SidebarTab::ColorAdjustments;
                                }
                                ActiveTool::Brush => {
                                    self.brush_settings.is_eraser = false;
                                }
                                ActiveTool::Eraser => {
                                    self.brush_settings.is_eraser = true;
                                }
                                _ => {}
                            }
                        }
                    };

                    tool_icon(ActiveTool::Select, "↖");
                    tool_icon(ActiveTool::Wand, "🪄");
                    tool_icon(ActiveTool::ColorGrading, "◐");
                    tool_icon(ActiveTool::Effects, "✻");
                    tool_icon(ActiveTool::Marquee, "⬚");
                    tool_icon(ActiveTool::Retouch, "🩹");
                    tool_icon(ActiveTool::Brush, "🖌");
                    tool_icon(ActiveTool::Eraser, "⌫");
                    tool_icon(ActiveTool::ColorPicker, "💧");
                    tool_icon(ActiveTool::Pen, "✒");
                    tool_icon(ActiveTool::Text, "T");
                    tool_icon(ActiveTool::Crop, "⯐");
                    tool_icon(ActiveTool::Select, "•••");
                });
            });

        // 5. FLOATING LEFT PANEL: "LAYERS" (Exactly matching the screenshot)
        if self.show_layers_panel {
            let layers_panel_width = 300.0;
            let layers_panel_height = 560.0;
            let layers_panel_rect = Rect::from_min_size(
                Pos2::new(total_rect.min.x + 16.0, total_rect.min.y + 16.0),
                Vec2::new(layers_panel_width, layers_panel_height),
            );

            let mut layers_ui = ui.new_child(egui::UiBuilder::new().max_rect(layers_panel_rect));
            egui::Frame::new()
                .fill(Color32::from_rgba_unmultiplied(28, 29, 34, 235))
                .corner_radius(CornerRadius::same(18))
                .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 20)))
                .inner_margin(12.0)
                .show(&mut layers_ui, |ui| {
                    // Header: '•••', '🎛', 'Layers', '✕'
                    ui.horizontal(|ui| {
                        let _ = ui.button(egui::RichText::new("•••").size(11.0));
                        let _ = ui.button(egui::RichText::new("🎛").size(12.0));

                        ui.add_space(30.0);
                        ui.label(
                            egui::RichText::new("Layers")
                                .size(15.0)
                                .strong()
                                .color(Color32::from_rgb(240, 240, 245)),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(egui::RichText::new("✕").size(12.0).color(Color32::from_rgb(160, 160, 170))).clicked() {
                                self.show_layers_panel = false;
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Layer Cards List
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 6.0);

                        // Render layers in hierarchy matching reference screenshot
                        for layer in &self.document.layers {
                            let is_selected = self.document.selected_layer_id == Some(layer.id);
                            let bg_card = if is_selected {
                                Color32::from_rgba_unmultiplied(52, 54, 63, 240)
                            } else {
                                Color32::from_rgba_unmultiplied(38, 39, 45, 180)
                            };

                            let card_frame = egui::Frame::new()
                                .fill(bg_card)
                                .corner_radius(CornerRadius::same(12))
                                .inner_margin(8.0);

                            card_frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // Branch arrow '↳' for indented sub-layers
                                    if layer.name.contains("Mask") || layer.name.contains("Ring") || layer.name.contains("Effect") {
                                        ui.label(
                                            egui::RichText::new("↳")
                                                .size(13.0)
                                                .color(Color32::from_rgb(140, 140, 150)),
                                        );
                                    }

                                    // Thumbnail glyph box
                                    let (thumb_rect, _resp) = ui.allocate_exact_size(Vec2::new(36.0, 36.0), Sense::click());
                                    ui.painter().rect_filled(
                                        thumb_rect,
                                        CornerRadius::same(6),
                                        Color32::from_rgb(24, 25, 29),
                                    );
                                    ui.painter().rect_stroke(
                                        thumb_rect,
                                        CornerRadius::same(6),
                                        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 15)),
                                        StrokeKind::Inside,
                                    );
                                    let glyph = if layer.name.contains("Background") {
                                        "▨"
                                    } else if layer.name.contains("Frame") {
                                        "⌜⌟"
                                    } else if layer.name.contains("Focus") {
                                        "◎"
                                    } else {
                                        "❐"
                                    };
                                    ui.painter().text(
                                        thumb_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        glyph,
                                        egui::FontId::proportional(14.0),
                                        Color32::from_rgb(200, 200, 210),
                                    );

                                    ui.add_space(4.0);

                                    // Title & subtitle
                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new(&layer.name)
                                                .size(13.0)
                                                .strong()
                                                .color(Color32::from_rgb(235, 235, 240)),
                                        );
                                        let sub = if layer.name.contains("Background") {
                                            "Grain, Color Fill"
                                        } else if layer.name.contains("Focus") {
                                            "Gaussian"
                                        } else {
                                            "900 × 900 px, Effects"
                                        };
                                        ui.label(
                                            egui::RichText::new(sub)
                                                .size(10.5)
                                                .color(Color32::from_rgb(140, 140, 150)),
                                        );
                                    });

                                    // Right controls: Lock icon or Chevron
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if layer.name.contains("Background") {
                                            ui.label(egui::RichText::new("🔒").size(11.0).color(Color32::from_rgb(150, 150, 160)));
                                        } else {
                                            ui.label(egui::RichText::new("›").size(14.0).color(Color32::from_rgb(150, 150, 160)));
                                        }
                                    });
                                });
                            });
                        }
                    });
                });
        }

        // 6. FLOATING RIGHT PANEL: "EFFECTS" (The centerpiece from the reference screenshot!)
        if self.show_effects_panel {
            let effects_panel_width = 320.0;
            let effects_panel_height = 580.0;
            let effects_panel_rect = Rect::from_min_size(
                Pos2::new(
                    total_rect.max.x - tool_strip_width - tool_strip_margin - effects_panel_width - 12.0,
                    total_rect.min.y + 16.0,
                ),
                Vec2::new(effects_panel_width, effects_panel_height),
            );

            let mut effects_ui = ui.new_child(egui::UiBuilder::new().max_rect(effects_panel_rect));
            egui::Frame::new()
                .fill(Color32::from_rgba_unmultiplied(28, 29, 34, 235))
                .corner_radius(CornerRadius::same(18))
                .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 20)))
                .inner_margin(14.0)
                .show(&mut effects_ui, |ui| {
                    // Header: '+', '•••', 'Effects', '✕'
                    ui.horizontal(|ui| {
                        let _ = ui.button(egui::RichText::new("+").size(14.0));
                        let _ = ui.button(egui::RichText::new("•••").size(11.0));

                        ui.add_space(38.0);
                        let title = match self.active_sidebar_tab {
                            SidebarTab::Effects => "Effects",
                            SidebarTab::ColorAdjustments => "Color",
                        };
                        ui.label(
                            egui::RichText::new(title)
                                .size(15.0)
                                .strong()
                                .color(Color32::from_rgb(240, 240, 245)),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(egui::RichText::new("✕").size(12.0).color(Color32::from_rgb(160, 160, 170))).clicked() {
                                self.show_effects_panel = false;
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Preset Selector Card (e.g. desert dunes thumbnail + '>')
                    egui::Frame::new()
                        .fill(Color32::from_rgba_unmultiplied(40, 41, 48, 180))
                        .corner_radius(CornerRadius::same(12))
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Preset")
                                        .size(13.0)
                                        .color(Color32::from_rgb(230, 230, 235)),
                                );

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new("›").size(14.0).color(Color32::from_rgb(160, 160, 170)));
                                    if let Some(tex) = &self.preset_texture {
                                        ui.image((tex.id(), Vec2::new(56.0, 32.0)));
                                    }
                                });
                            });
                        });

                    ui.add_space(8.0);

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 10.0);

                        // EFFECT BLOCK 1: VIGNETTE (Exactly from screenshot!)
                        egui::Frame::new()
                            .fill(Color32::from_rgba_unmultiplied(38, 39, 45, 180))
                            .corner_radius(CornerRadius::same(14))
                            .inner_margin(12.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("Vignette")
                                            .size(14.0)
                                            .strong()
                                            .color(Color32::from_rgb(240, 240, 245)),
                                    );

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if Self::draw_toggle_switch(ui, &mut self.vignette_enabled) {
                                            self.needs_recomposite = true;
                                        }
                                        ui.add_space(4.0);
                                        let _ = ui.small_button(egui::RichText::new("🗑").size(12.0));
                                        let _ = ui.small_button(egui::RichText::new("◎").size(12.0));
                                    });
                                });

                                if self.vignette_enabled {
                                    ui.add_space(8.0);

                                    // Radius Slider
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Radius").size(12.5).color(Color32::from_rgb(190, 190, 200)));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(format!("{:.0}%", self.vignette_radius * 100.0));
                                        });
                                    });
                                    if ui.add(egui::Slider::new(&mut self.vignette_radius, 0.0..=1.0).show_value(false)).changed() {
                                        self.needs_recomposite = true;
                                    }

                                    ui.add_space(4.0);

                                    // Intensity Slider
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Intensity").size(12.5).color(Color32::from_rgb(190, 190, 200)));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(format!("{:.0}%", self.vignette_intensity * 100.0));
                                        });
                                    });
                                    if ui.add(egui::Slider::new(&mut self.vignette_intensity, 0.0..=1.0).show_value(false)).changed() {
                                        self.needs_recomposite = true;
                                    }

                                    ui.add_space(4.0);

                                    // Falloff Slider
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Falloff").size(12.5).color(Color32::from_rgb(190, 190, 200)));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(format!("{:.0}%", self.vignette_falloff * 100.0));
                                        });
                                    });
                                    if ui.add(egui::Slider::new(&mut self.vignette_falloff, 0.0..=1.0).show_value(false)).changed() {
                                        self.needs_recomposite = true;
                                    }
                                }
                            });

                        // EFFECT BLOCK 2: FOCUS / BLUR (Exactly from screenshot!)
                        egui::Frame::new()
                            .fill(Color32::from_rgba_unmultiplied(38, 39, 45, 180))
                            .corner_radius(CornerRadius::same(14))
                            .inner_margin(12.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("Focus")
                                            .size(14.0)
                                            .strong()
                                            .color(Color32::from_rgb(240, 240, 245)),
                                    );

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if Self::draw_toggle_switch(ui, &mut self.focus_enabled) {
                                            self.needs_recomposite = true;
                                        }
                                        ui.add_space(4.0);
                                        let _ = ui.small_button(egui::RichText::new("🗑").size(12.0));
                                        let _ = ui.small_button(egui::RichText::new("◎").size(12.0));
                                    });
                                });

                                if self.focus_enabled {
                                    ui.add_space(8.0);

                                    // Blur Slider
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Blur").size(12.5).color(Color32::from_rgb(190, 190, 200)));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(format!("{:.1} px", self.focus_blur));
                                        });
                                    });
                                    if ui.add(egui::Slider::new(&mut self.focus_blur, 0.0..=20.0).show_value(false)).changed() {
                                        self.needs_recomposite = true;
                                    }

                                    ui.add_space(4.0);

                                    // Transition Slider
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Transition").size(12.5).color(Color32::from_rgb(190, 190, 200)));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(format!("{:.0}%", self.focus_transition * 100.0));
                                        });
                                    });
                                    if ui.add(egui::Slider::new(&mut self.focus_transition, 0.0..=1.0).show_value(false)).changed() {
                                        self.needs_recomposite = true;
                                    }
                                }
                            });

                        // EFFECT BLOCK 3: COLOR ADJUSTMENTS (If Color Grading tab active)
                        if self.active_sidebar_tab == SidebarTab::ColorAdjustments {
                            egui::Frame::new()
                                .fill(Color32::from_rgba_unmultiplied(38, 39, 45, 180))
                                .corner_radius(CornerRadius::same(14))
                                .inner_margin(12.0)
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Color Adjustments")
                                            .size(14.0)
                                            .strong()
                                            .color(Color32::from_rgb(240, 240, 245)),
                                    );
                                    ui.add_space(6.0);

                                    let mut changed = false;

                                    ui.horizontal(|ui| {
                                        ui.label("Exposure:");
                                        changed |= ui.add(egui::Slider::new(&mut self.color_adjustments.exposure, -3.0..=3.0).suffix(" EV")).changed();
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Brightness:");
                                        changed |= ui.add(egui::Slider::new(&mut self.color_adjustments.brightness, -0.8..=0.8)).changed();
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Contrast:");
                                        changed |= ui.add(egui::Slider::new(&mut self.color_adjustments.contrast, -0.8..=0.8)).changed();
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Saturation:");
                                        changed |= ui.add(egui::Slider::new(&mut self.color_adjustments.saturation, -1.0..=1.0)).changed();
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Temperature:");
                                        changed |= ui.add(egui::Slider::new(&mut self.color_adjustments.temperature, -1.0..=1.0)).changed();
                                    });

                                    if changed {
                                        self.needs_recomposite = true;
                                    }
                                });
                        }
                    });
                });
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 840.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("OxyLayer Pro — Creative Studio"),
        ..Default::default()
    };

    eframe::run_native(
        "oxylayer",
        options,
        Box::new(|cc| Ok(Box::new(OxyLayerApp::new(cc)))),
    )
}
