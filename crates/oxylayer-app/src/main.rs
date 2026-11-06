use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Vec2};
use oxylayer_brush::{BrushEngine, BrushSettings};
use oxylayer_core::{
    BlendMode, Document, HistoryManager, Layer, LayerId, PixelBuffer, Rgba8,
};
use oxylayer_filter::{gaussian_blur, pixelate, sharpen, vignette, ColorAdjustments};
use oxylayer_vector::VectorElement;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTool {
    Move,
    Brush,
    Eraser,
    ShapeRectangle,
    ShapeCircle,
    ShapeStar,
    Text,
}

pub struct OxyLayerApp {
    document: Document,
    history: HistoryManager,
    active_tool: ActiveTool,
    brush_settings: BrushSettings,
    color_adjustments: ColorAdjustments,
    preview_adjustments: bool,
    filter_blur_radius: f32,
    filter_sharpen_amount: f32,
    filter_pixelate_block: u32,
    filter_vignette_radius: f32,
    filter_vignette_softness: f32,

    // Viewport state
    zoom: f32,
    pan: Vec2,
    last_mouse_pos: Option<glam::Vec2>,
    brush_residual_dist: f32,

    // Composite render cache
    canvas_texture: Option<egui::TextureHandle>,
    needs_recomposite: bool,

    // Status notification
    status_message: String,
    status_timer: f32,
}

impl OxyLayerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let width = 1000;
        let height = 700;
        let document = Document::new("Untitled Artwork", width, height);
        let history = HistoryManager::new(50);

        Self {
            document,
            history,
            active_tool: ActiveTool::Brush,
            brush_settings: BrushSettings {
                size: 25.0,
                hardness: 0.75,
                opacity: 1.0,
                flow: 1.0,
                spacing: 0.15,
                color: Rgba8::new(10, 132, 255, 255), // macOS vibrant blue
                is_eraser: false,
            },
            color_adjustments: ColorAdjustments::default(),
            preview_adjustments: true,
            filter_blur_radius: 0.0,
            filter_sharpen_amount: 0.0,
            filter_pixelate_block: 1,
            filter_vignette_radius: 0.7,
            filter_vignette_softness: 0.4,

            zoom: 1.0,
            pan: Vec2::ZERO,
            last_mouse_pos: None,
            brush_residual_dist: 0.0,

            canvas_texture: None,
            needs_recomposite: true,

            status_message: "Ready — Welcome to OxyLayer Pro".to_string(),
            status_timer: 3.0,
        }
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
        self.status_timer = 4.0;
    }

    /// Rerenders the document layer composite into an egui TextureHandle
    fn update_texture(&mut self, ctx: &egui::Context) {
        let mut composite = self.document.composite();

        // Apply non-destructive color grading preview if active
        if self.preview_adjustments && !self.color_adjustments.is_identity() {
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

        self.needs_recomposite = false;
    }

    /// Apply color adjustments permanently to the selected layer
    fn commit_color_adjustments(&mut self) {
        if self.color_adjustments.is_identity() {
            return;
        }

        self.history.push_snapshot(&self.document);
        if let Some(layer) = self.document.selected_layer_mut() {
            if let Some(buf) = layer.as_raster_mut() {
                self.color_adjustments.apply_to_buffer(buf);
                self.color_adjustments = ColorAdjustments::default();
                self.needs_recomposite = true;
                self.set_status("Color adjustments applied to layer");
            }
        }
    }

    /// Apply blur filter to the selected layer
    fn apply_blur(&mut self) {
        if self.filter_blur_radius <= 0.1 {
            return;
        }
        self.history.push_snapshot(&self.document);
        if let Some(layer) = self.document.selected_layer_mut() {
            if let Some(buf) = layer.as_raster_mut() {
                let blurred = gaussian_blur(buf, self.filter_blur_radius);
                *buf = blurred;
                self.needs_recomposite = true;
                self.set_status(format!("Applied Gaussian Blur ({:.1}px)", self.filter_blur_radius));
            }
        }
    }

    /// Apply sharpen filter to the selected layer
    fn apply_sharpen(&mut self) {
        if self.filter_sharpen_amount <= 0.05 {
            return;
        }
        self.history.push_snapshot(&self.document);
        if let Some(layer) = self.document.selected_layer_mut() {
            if let Some(buf) = layer.as_raster_mut() {
                let sharpened = sharpen(buf, self.filter_sharpen_amount);
                *buf = sharpened;
                self.needs_recomposite = true;
                self.set_status(format!("Applied Sharpen ({:.1})", self.filter_sharpen_amount));
            }
        }
    }

    /// Apply pixelate filter to the selected layer
    fn apply_pixelate(&mut self) {
        if self.filter_pixelate_block <= 1 {
            return;
        }
        self.history.push_snapshot(&self.document);
        if let Some(layer) = self.document.selected_layer_mut() {
            if let Some(buf) = layer.as_raster_mut() {
                let pix = pixelate(buf, self.filter_pixelate_block);
                *buf = pix;
                self.needs_recomposite = true;
                self.set_status(format!("Applied Pixelate ({}px)", self.filter_pixelate_block));
            }
        }
    }

    /// Apply vignette to the selected layer
    fn apply_vignette(&mut self) {
        self.history.push_snapshot(&self.document);
        if let Some(layer) = self.document.selected_layer_mut() {
            if let Some(buf) = layer.as_raster_mut() {
                vignette(buf, self.filter_vignette_radius, self.filter_vignette_softness);
                self.needs_recomposite = true;
                self.set_status("Applied Vignette filter");
            }
        }
    }

    /// Stamp or draw vector shape onto active raster layer
    fn add_shape_to_layer(&mut self, shape_kind: ActiveTool, center_x: f64, center_y: f64) {
        self.history.push_snapshot(&self.document);
        let color = self.brush_settings.color;

        let elem = match shape_kind {
            ActiveTool::ShapeRectangle => VectorElement {
                fill_color: Some(color),
                ..VectorElement::new_rectangle(center_x - 60.0, center_y - 40.0, 120.0, 80.0, 8.0)
            },
            ActiveTool::ShapeCircle => VectorElement {
                fill_color: Some(color),
                ..VectorElement::new_circle(center_x, center_y, 50.0)
            },
            ActiveTool::ShapeStar => VectorElement {
                fill_color: Some(color),
                ..VectorElement::new_star(center_x, center_y, 5, 25.0, 55.0)
            },
            _ => return,
        };

        if let Some(layer) = self.document.selected_layer_mut() {
            if let Some(buf) = layer.as_raster_mut() {
                elem.rasterize(buf);
                self.needs_recomposite = true;
                self.set_status("Added vector shape to layer");
            }
        }
    }
}

impl eframe::App for OxyLayerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Pixelmator Pro Dark Theme styling
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(26, 26, 28);
        visuals.window_fill = Color32::from_rgb(32, 32, 35);
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(36, 36, 40);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(44, 44, 48);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(60, 60, 68);
        visuals.widgets.active.bg_fill = Color32::from_rgb(10, 132, 255);
        visuals.selection.bg_fill = Color32::from_rgb(10, 132, 255);
        ctx.set_visuals(visuals);

        // Top App Bar: Menu, Title, Canvas Zoom info, Undo/Redo
        egui::Panel::top("top_header_panel")
            .frame(egui::Frame::new().fill(Color32::from_rgb(20, 20, 22)).inner_margin(8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("✦ OxyLayer")
                            .color(Color32::from_rgb(10, 132, 255))
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("PRO")
                            .small()
                            .color(Color32::from_rgb(160, 160, 170)),
                    );

                    ui.separator();

                    if ui.button("📄 New").clicked() {
                        self.history.push_snapshot(&self.document);
                        self.document = Document::new("New Document", 1000, 700);
                        self.needs_recomposite = true;
                        self.set_status("Created new document (1000x700)");
                    }

                    if ui.button("💾 Save Project (.oxy)").clicked() {
                        let path = PathBuf::from("artwork.oxy");
                        if let Err(e) = oxylayer_io::ProjectFile::save(&self.document, &path) {
                            self.set_status(format!("Save error: {}", e));
                        } else {
                            self.set_status("Saved project to artwork.oxy");
                        }
                    }

                    if ui.button("📤 Export PNG").clicked() {
                        let composite = self.document.composite();
                        let path = PathBuf::from("export.png");
                        if let Err(e) = oxylayer_io::ImageCodec::save_png(&composite, &path) {
                            self.set_status(format!("Export error: {}", e));
                        } else {
                            self.set_status("Exported PNG successfully to export.png");
                        }
                    }

                    if ui.button("📤 Export JPEG").clicked() {
                        let composite = self.document.composite();
                        let path = PathBuf::from("export.jpg");
                        if let Err(e) = oxylayer_io::ImageCodec::save_jpeg(&composite, &path, 90) {
                            self.set_status(format!("Export error: {}", e));
                        } else {
                            self.set_status("Exported JPEG to export.jpg");
                        }
                    }

                    ui.separator();

                    // Undo / Redo buttons
                    let can_undo = self.history.can_undo();
                    if ui.add_enabled(can_undo, egui::Button::new("↶ Undo")).clicked() {
                        if self.history.undo(&mut self.document) {
                            self.needs_recomposite = true;
                            self.set_status("Undo performed");
                        }
                    }

                    let can_redo = self.history.can_redo();
                    if ui.add_enabled(can_redo, egui::Button::new("↷ Redo")).clicked() {
                        if self.history.redo(&mut self.document) {
                            self.needs_recomposite = true;
                            self.set_status("Redo performed");
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!(
                            "{} × {} px | {:.0}%",
                            self.document.width,
                            self.document.height,
                            self.zoom * 100.0
                        ));
                        if ui.button("100%").clicked() {
                            self.zoom = 1.0;
                            self.pan = Vec2::ZERO;
                        }
                        if ui.button("Fit").clicked() {
                            self.zoom = 0.8;
                            self.pan = Vec2::ZERO;
                        }
                    });
                });
            });

        // Bottom Status Bar
        egui::Panel::bottom("status_bar")
            .frame(egui::Frame::new().fill(Color32::from_rgb(18, 18, 20)).inner_margin(4.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&self.status_message)
                            .small()
                            .color(Color32::from_rgb(200, 200, 210)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Pure Rust Engine • SIMD/Rayon • Cross-Platform")
                                .small()
                                .color(Color32::from_rgb(120, 120, 130)),
                        );
                    });
                });
            });

        // Left Side: Tools Palette
        egui::Panel::left("tools_palette")
            .resizable(false)
            .default_size(52.0)
            .frame(egui::Frame::new().fill(Color32::from_rgb(24, 24, 26)).inner_margin(6.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);

                    let tool_btn = |ui: &mut egui::Ui, name: &str, tool: ActiveTool, current: ActiveTool| {
                        let selected = tool == current;
                        let text = egui::RichText::new(name).size(16.0);
                        let btn = egui::Button::new(text).selected(selected).min_size(Vec2::new(40.0, 36.0));
                        if ui.add(btn).clicked() {
                            return Some(tool);
                        }
                        None
                    };

                    if let Some(t) = tool_btn(ui, "↖", ActiveTool::Move, self.active_tool) {
                        self.active_tool = t;
                    }
                    if let Some(t) = tool_btn(ui, "🖌", ActiveTool::Brush, self.active_tool) {
                        self.active_tool = t;
                        self.brush_settings.is_eraser = false;
                    }
                    if let Some(t) = tool_btn(ui, "⌫", ActiveTool::Eraser, self.active_tool) {
                        self.active_tool = t;
                        self.brush_settings.is_eraser = true;
                    }
                    ui.separator();
                    if let Some(t) = tool_btn(ui, "▭", ActiveTool::ShapeRectangle, self.active_tool) {
                        self.active_tool = t;
                    }
                    if let Some(t) = tool_btn(ui, "○", ActiveTool::ShapeCircle, self.active_tool) {
                        self.active_tool = t;
                    }
                    if let Some(t) = tool_btn(ui, "★", ActiveTool::ShapeStar, self.active_tool) {
                        self.active_tool = t;
                    }
                    ui.separator();
                    if let Some(t) = tool_btn(ui, "T", ActiveTool::Text, self.active_tool) {
                        self.active_tool = t;
                    }

                    ui.add_space(20.0);
                    // Active color swatch
                    let col = self.brush_settings.color;
                    let (rect, _response) = ui.allocate_exact_size(Vec2::new(28.0, 28.0), egui::Sense::click());
                    ui.painter().rect_filled(
                        rect,
                        CornerRadius::same(6),
                        Color32::from_rgba_unmultiplied(col.r, col.g, col.b, col.a),
                    );
                    ui.painter().rect_stroke(
                        rect,
                        CornerRadius::same(6),
                        Stroke::new(1.5, Color32::from_rgb(180, 180, 190)),
                        StrokeKind::Inside,
                    );
                });
            });

        // Right Side: Inspector Sidebar (Layers + Color Grading + Filters + Tool Dynamics)
        egui::Panel::right("inspector_sidebar")
            .resizable(true)
            .default_size(320.0)
            .frame(egui::Frame::new().fill(Color32::from_rgb(26, 26, 28)).inner_margin(12.0))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Section 1: Layers Panel
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("LAYERS")
                                .strong()
                                .color(Color32::from_rgb(180, 180, 190)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("+ New").clicked() {
                                self.history.push_snapshot(&self.document);
                                let w = self.document.width;
                                let h = self.document.height;
                                let count = self.document.layer_count() + 1;
                                let layer = Layer::new_raster(
                                    format!("Layer {}", count),
                                    PixelBuffer::new(w, h),
                                );
                                self.document.add_layer(layer);
                                self.needs_recomposite = true;
                                self.set_status(format!("Added Layer {}", count));
                            }
                        });
                    });

                    ui.add_space(4.0);

                    // Layer Stack list (rendered top-to-bottom)
                    let layer_ids: Vec<LayerId> = self.document.layers.iter().map(|l| l.id).rev().collect();
                    for id in layer_ids {
                        let is_selected = self.document.selected_layer_id == Some(id);
                        let layer_opt = self.document.find_layer(id);
                        if layer_opt.is_none() {
                            continue;
                        }
                        let (name, visible, opacity, blend) = {
                            let l = layer_opt.unwrap();
                            (l.name.clone(), l.visible, l.opacity, l.blend_mode)
                        };

                        let bg_color = if is_selected {
                            Color32::from_rgb(40, 50, 70)
                        } else {
                            Color32::from_rgb(32, 32, 35)
                        };

                        egui::Frame::new()
                            .fill(bg_color)
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(6.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // Visibility toggle
                                    let mut vis = visible;
                                    if ui.checkbox(&mut vis, "").changed() {
                                        if let Some(l) = self.document.find_layer_mut(id) {
                                            l.visible = vis;
                                            self.needs_recomposite = true;
                                        }
                                    }

                                    // Layer name selection
                                    if ui.selectable_label(is_selected, &name).clicked() {
                                        self.document.selected_layer_id = Some(id);
                                    }

                                    // Duplicate / Delete shortcuts
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.small_button("🗑").clicked() {
                                            self.history.push_snapshot(&self.document);
                                            self.document.remove_layer(id);
                                            self.needs_recomposite = true;
                                        }
                                        if ui.small_button("📋").clicked() {
                                            self.history.push_snapshot(&self.document);
                                            self.document.duplicate_layer(id);
                                            self.needs_recomposite = true;
                                        }
                                    });
                                });

                                // Layer Blend Mode and Opacity (if selected)
                                if is_selected {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Blend:").small());
                                        egui::ComboBox::from_id_salt(format!("blend_{:?}", id))
                                            .selected_text(blend.display_name())
                                            .show_ui(ui, |ui| {
                                                for mode in BlendMode::ALL {
                                                    let mut current = blend;
                                                    if ui.selectable_value(&mut current, mode, mode.display_name()).clicked() {
                                                        if let Some(l) = self.document.find_layer_mut(id) {
                                                            l.blend_mode = current;
                                                            self.needs_recomposite = true;
                                                        }
                                                    }
                                                }
                                            });
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Opacity:").small());
                                        let mut op = opacity * 100.0;
                                        if ui.add(egui::Slider::new(&mut op, 0.0..=100.0).suffix("%")).changed() {
                                            if let Some(l) = self.document.find_layer_mut(id) {
                                                l.opacity = op / 100.0;
                                                self.needs_recomposite = true;
                                            }
                                        }
                                    });
                                }
                            });
                        ui.add_space(2.0);
                    }

                    ui.separator();

                    // Section 2: Tool Settings (Brush Dynamics)
                    if matches!(self.active_tool, ActiveTool::Brush | ActiveTool::Eraser) {
                        ui.label(
                            egui::RichText::new("BRUSH DYNAMICS")
                                .strong()
                                .color(Color32::from_rgb(180, 180, 190)),
                        );
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label("Size:");
                            ui.add(egui::Slider::new(&mut self.brush_settings.size, 1.0..=200.0).suffix(" px"));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Hardness:");
                            ui.add(egui::Slider::new(&mut self.brush_settings.hardness, 0.0..=1.0));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Opacity:");
                            let mut op = self.brush_settings.opacity * 100.0;
                            if ui.add(egui::Slider::new(&mut op, 1.0..=100.0).suffix("%")).changed() {
                                self.brush_settings.opacity = op / 100.0;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Flow:");
                            let mut flow = self.brush_settings.flow * 100.0;
                            if ui.add(egui::Slider::new(&mut flow, 1.0..=100.0).suffix("%")).changed() {
                                self.brush_settings.flow = flow / 100.0;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Color:");
                            let mut c = [
                                self.brush_settings.color.r,
                                self.brush_settings.color.g,
                                self.brush_settings.color.b,
                            ];
                            if ui.color_edit_button_srgb(&mut c).changed() {
                                self.brush_settings.color = Rgba8::new(c[0], c[1], c[2], 255);
                            }
                        });

                        ui.separator();
                    }

                    // Section 3: Non-destructive Color Adjustments (Pixelmator Pro parity)
                    ui.label(
                        egui::RichText::new("COLOR ADJUSTMENTS")
                            .strong()
                            .color(Color32::from_rgb(180, 180, 190)),
                    );
                    ui.add_space(4.0);

                    let mut changed_adj = false;

                    ui.horizontal(|ui| {
                        ui.label("Exposure:");
                        changed_adj |= ui.add(egui::Slider::new(&mut self.color_adjustments.exposure, -3.0..=3.0).suffix(" EV")).changed();
                    });

                    ui.horizontal(|ui| {
                        ui.label("Brightness:");
                        changed_adj |= ui.add(egui::Slider::new(&mut self.color_adjustments.brightness, -0.8..=0.8)).changed();
                    });

                    ui.horizontal(|ui| {
                        ui.label("Contrast:");
                        changed_adj |= ui.add(egui::Slider::new(&mut self.color_adjustments.contrast, -0.8..=0.8)).changed();
                    });

                    ui.horizontal(|ui| {
                        ui.label("Saturation:");
                        changed_adj |= ui.add(egui::Slider::new(&mut self.color_adjustments.saturation, -1.0..=1.0)).changed();
                    });

                    ui.horizontal(|ui| {
                        ui.label("Vibrance:");
                        changed_adj |= ui.add(egui::Slider::new(&mut self.color_adjustments.vibrance, -1.0..=1.0)).changed();
                    });

                    ui.horizontal(|ui| {
                        ui.label("Temperature:");
                        changed_adj |= ui.add(egui::Slider::new(&mut self.color_adjustments.temperature, -1.0..=1.0)).changed();
                    });

                    ui.horizontal(|ui| {
                        ui.label("Tint:");
                        changed_adj |= ui.add(egui::Slider::new(&mut self.color_adjustments.tint, -1.0..=1.0)).changed();
                    });

                    ui.horizontal(|ui| {
                        ui.label("Highlights:");
                        changed_adj |= ui.add(egui::Slider::new(&mut self.color_adjustments.highlights, -1.0..=1.0)).changed();
                    });

                    ui.horizontal(|ui| {
                        ui.label("Shadows:");
                        changed_adj |= ui.add(egui::Slider::new(&mut self.color_adjustments.shadows, -1.0..=1.0)).changed();
                    });

                    ui.horizontal(|ui| {
                        changed_adj |= ui.checkbox(&mut self.color_adjustments.black_and_white, "Black & White").changed();
                        changed_adj |= ui.checkbox(&mut self.color_adjustments.invert, "Invert").changed();
                    });

                    if changed_adj {
                        self.needs_recomposite = true;
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Commit to Layer").clicked() {
                            self.commit_color_adjustments();
                        }
                        if ui.button("Reset").clicked() {
                            self.color_adjustments = ColorAdjustments::default();
                            self.needs_recomposite = true;
                        }
                    });

                    ui.separator();

                    // Section 4: Filters & Effects
                    ui.label(
                        egui::RichText::new("EFFECTS & FILTERS")
                            .strong()
                            .color(Color32::from_rgb(180, 180, 190)),
                    );
                    ui.add_space(4.0);

                    // Gaussian blur
                    ui.horizontal(|ui| {
                        ui.label("Gaussian Blur:");
                        ui.add(egui::Slider::new(&mut self.filter_blur_radius, 0.0..=25.0).suffix(" px"));
                    });
                    if ui.button("Apply Blur").clicked() {
                        self.apply_blur();
                    }

                    // Sharpen
                    ui.horizontal(|ui| {
                        ui.label("Sharpen:");
                        ui.add(egui::Slider::new(&mut self.filter_sharpen_amount, 0.0..=3.0));
                    });
                    if ui.button("Apply Sharpen").clicked() {
                        self.apply_sharpen();
                    }

                    // Vignette
                    ui.horizontal(|ui| {
                        ui.label("Vignette:");
                        ui.add(egui::Slider::new(&mut self.filter_vignette_radius, 0.1..=1.0));
                    });
                    if ui.button("Apply Vignette").clicked() {
                        self.apply_vignette();
                    }

                    // Pixelate
                    ui.horizontal(|ui| {
                        ui.label("Pixelate Block:");
                        ui.add(egui::Slider::new(&mut self.filter_pixelate_block, 1..=40).suffix(" px"));
                    });
                    if ui.button("Apply Pixelate").clicked() {
                        self.apply_pixelate();
                    }
                });
            });

        // Center: Interactive Infinite Canvas Viewport
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::from_rgb(15, 15, 16)))
            .show(ui, |ui| {
                let (viewport_rect, response) =
                    ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

                // Pan and Zoom with middle click, space drag, or scroll wheel
                let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll_delta != 0.0 && viewport_rect.contains(response.hover_pos().unwrap_or_default()) {
                    let zoom_factor = if scroll_delta > 0.0 { 1.1 } else { 0.9 };
                    self.zoom = (self.zoom * zoom_factor).clamp(0.05, 30.0);
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

                // 1. Draw Checkerboard background pattern for canvas transparency
                let painter = ui.painter_at(viewport_rect);
                let checker_size = 12.0;
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
                                Color32::from_rgb(45, 45, 48)
                            } else {
                                Color32::from_rgb(35, 35, 38)
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

                // 2. Recomposite if dirty
                if self.needs_recomposite || self.canvas_texture.is_none() {
                    self.update_texture(&ctx);
                }

                // 3. Draw document texture
                if let Some(texture) = &self.canvas_texture {
                    painter.image(
                        texture.id(),
                        canvas_rect,
                        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }

                // 4. Draw canvas border / drop shadow
                painter.rect_stroke(
                    canvas_rect,
                    CornerRadius::ZERO,
                    Stroke::new(1.0, Color32::from_rgb(80, 80, 90)),
                    StrokeKind::Inside,
                );

                // 5. Handle drawing / interacting on canvas
                let pointer_pos = response.hover_pos();
                let is_drawing = response.dragged_by(egui::PointerButton::Primary)
                    && !ui.input(|i| i.key_down(egui::Key::Space));

                if is_drawing {
                    if let Some(pos) = pointer_pos {
                        if canvas_rect.contains(pos) {
                            // Map screen coordinate to canvas pixel coordinate
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
                                        // First dab
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

                // Click to stamp vector shapes
                if response.clicked_by(egui::PointerButton::Primary)
                    && !ui.input(|i| i.key_down(egui::Key::Space))
                {
                    if let Some(pos) = pointer_pos {
                        if canvas_rect.contains(pos) {
                            let norm_x = (pos.x - canvas_rect.min.x) / canvas_rect.width();
                            let norm_y = (pos.y - canvas_rect.min.y) / canvas_rect.height();
                            let cx = (norm_x * doc_w) as f64;
                            let cy = (norm_y * doc_h) as f64;

                            match self.active_tool {
                                ActiveTool::ShapeRectangle
                                | ActiveTool::ShapeCircle
                                | ActiveTool::ShapeStar => {
                                    self.add_shape_to_layer(self.active_tool, cx, cy);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("OxyLayer Pro — High-Performance Creative Studio"),
        ..Default::default()
    };

    eframe::run_native(
        "oxylayer",
        options,
        Box::new(|cc| Ok(Box::new(OxyLayerApp::new(cc)))),
    )
}
