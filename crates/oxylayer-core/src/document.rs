use crate::color::Rgba8;
use crate::buffer::PixelBuffer;
use crate::layer::{Layer, LayerId, LayerKind};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub dpi: f32,
    pub background_color: Option<Rgba8>,
    pub layers: Vec<Layer>,
    pub selected_layer_id: Option<LayerId>,
}

impl Document {
    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Self {
        let mut doc = Self {
            id: Uuid::new_v4(),
            title: title.into(),
            width,
            height,
            dpi: 72.0,
            background_color: Some(Rgba8::WHITE),
            layers: Vec::new(),
            selected_layer_id: None,
        };

        // Create default background layer
        let bg_layer = Layer::new_raster("Background", PixelBuffer::filled(width, height, Rgba8::WHITE));
        let bg_id = bg_layer.id;
        doc.layers.push(bg_layer);
        doc.selected_layer_id = Some(bg_id);

        doc
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn add_layer(&mut self, layer: Layer) -> LayerId {
        let id = layer.id;
        self.layers.push(layer);
        self.selected_layer_id = Some(id);
        id
    }

    pub fn selected_layer(&self) -> Option<&Layer> {
        let id = self.selected_layer_id?;
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn selected_layer_mut(&mut self) -> Option<&mut Layer> {
        let id = self.selected_layer_id?;
        self.layers.iter_mut().find(|l| l.id == id)
    }

    pub fn find_layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn find_layer_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    pub fn remove_layer(&mut self, id: LayerId) -> Option<Layer> {
        if let Some(pos) = self.layers.iter().position(|l| l.id == id) {
            let removed = self.layers.remove(pos);
            if self.selected_layer_id == Some(id) {
                self.selected_layer_id = self.layers.last().map(|l| l.id);
            }
            Some(removed)
        } else {
            None
        }
    }

    pub fn duplicate_layer(&mut self, id: LayerId) -> Option<LayerId> {
        if let Some(pos) = self.layers.iter().position(|l| l.id == id) {
            let mut clone = self.layers[pos].clone();
            clone.id = LayerId::new();
            clone.name = format!("{} copy", clone.name);
            let new_id = clone.id;
            self.layers.insert(pos + 1, clone);
            self.selected_layer_id = Some(new_id);
            Some(new_id)
        } else {
            None
        }
    }

    pub fn move_layer_up(&mut self, id: LayerId) -> bool {
        if let Some(pos) = self.layers.iter().position(|l| l.id == id) {
            if pos + 1 < self.layers.len() {
                self.layers.swap(pos, pos + 1);
                return true;
            }
        }
        false
    }

    pub fn move_layer_down(&mut self, id: LayerId) -> bool {
        if let Some(pos) = self.layers.iter().position(|l| l.id == id) {
            if pos > 0 {
                self.layers.swap(pos, pos - 1);
                return true;
            }
        }
        false
    }

    /// Parallel non-destructive document compositor
    pub fn composite(&self) -> PixelBuffer {
        let w = self.width;
        let h = self.height;

        let mut output = match self.background_color {
            Some(bg) => PixelBuffer::filled(w, h, bg),
            None => PixelBuffer::new(w, h),
        };

        for layer in &self.layers {
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }

            match &layer.kind {
                LayerKind::Raster(buf) => {
                    self.composite_raster_layer(&mut output, layer, buf);
                }
                LayerKind::Text(_) => {
                    // In raster composite, text rendered via vector/font rasterizer if present
                }
                LayerKind::Group(_) => {}
            }
        }

        output
    }

    fn composite_raster_layer(&self, output: &mut PixelBuffer, layer: &Layer, buf: &PixelBuffer) {
        let doc_w = self.width;
        let doc_h = self.height;
        let buf_w = buf.width();
        let buf_h = buf.height();

        if buf_w == 0 || buf_h == 0 {
            return;
        }

        // Fast path: 1:1 identity transform at (0, 0) without rotation
        let is_identity = layer.transform.translation == glam::Vec2::ZERO
            && layer.transform.rotation == 0.0
            && layer.transform.scale == glam::Vec2::ONE
            && buf_w == doc_w
            && buf_h == doc_h;

        let blend_mode = layer.blend_mode;
        let opacity = layer.opacity;

        if is_identity {
            let out_data = output.as_raw_mut();
            let src_data = buf.as_raw();

            out_data
                .par_chunks_exact_mut(4)
                .zip(src_data.par_chunks_exact(4))
                .for_each(|(dst_chunk, src_chunk)| {
                    let dst_px = Rgba8 {
                        r: dst_chunk[0],
                        g: dst_chunk[1],
                        b: dst_chunk[2],
                        a: dst_chunk[3],
                    };
                    let src_px = Rgba8 {
                        r: src_chunk[0],
                        g: src_chunk[1],
                        b: src_chunk[2],
                        a: src_chunk[3],
                    };

                    let res = blend_mode.blend_u8(dst_px, src_px, opacity);
                    dst_chunk[0] = res.r;
                    dst_chunk[1] = res.g;
                    dst_chunk[2] = res.b;
                    dst_chunk[3] = res.a;
                });
        } else {
            // General transformed sampling using inverse 2D affine matrix
            let inv_opt = layer.transform.to_inverse_affine2(buf_w as f32, buf_h as f32);
            let Some(inv) = inv_opt else { return };

            let out_data = output.as_raw_mut();

            out_data
                .par_chunks_exact_mut((doc_w * 4) as usize)
                .enumerate()
                .for_each(|(y_idx, row_chunk)| {
                    let cy = y_idx as f32;
                    for x_idx in 0..doc_w {
                        let cx = x_idx as f32;
                        let src_pt = inv.transform_point2(glam::Vec2::new(cx, cy));

                        if src_pt.x >= 0.0
                            && src_pt.x < buf_w as f32
                            && src_pt.y >= 0.0
                            && src_pt.y < buf_h as f32
                        {
                            let u = src_pt.x / (buf_w - 1).max(1) as f32;
                            let v = src_pt.y / (buf_h - 1).max(1) as f32;
                            let sampled = buf.sample_bilinear(u, v);

                            let dst_offset = (x_idx * 4) as usize;
                            let dst_px = Rgba8 {
                                r: row_chunk[dst_offset],
                                g: row_chunk[dst_offset + 1],
                                b: row_chunk[dst_offset + 2],
                                a: row_chunk[dst_offset + 3],
                            };

                            let res = blend_mode
                                .blend(dst_px.to_f32(), sampled, opacity)
                                .to_rgba8();

                            row_chunk[dst_offset] = res.r;
                            row_chunk[dst_offset + 1] = res.g;
                            row_chunk[dst_offset + 2] = res.b;
                            row_chunk[dst_offset + 3] = res.a;
                        }
                    }
                });
        }
    }
}
