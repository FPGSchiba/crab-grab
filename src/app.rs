use std::borrow::Cow;
use arboard::{Clipboard, ImageData};
use eframe::egui;
use image::RgbaImage;
use crate::utils;


pub struct CrabGrabApp {
    raw_image: RgbaImage,
    // CHANGED: We now store multiple tiles instead of one texture
    tiles: Vec<(f32, egui::TextureHandle)>,
    start_pos: Option<egui::Pos2>,
    current_pos: Option<egui::Pos2>,
}
impl CrabGrabApp {
    pub fn new(cc: &eframe::CreationContext, raw_image: RgbaImage) -> Self {
        // Load the image as tiles immediately
        let tiles = utils::load_image_as_tiles(&cc.egui_ctx, &raw_image);

        Self {
            raw_image,
            tiles, // Store the list
            start_pos: None,
            current_pos: None,
        }
    }

    fn handle_capture_finish(&mut self, rect: egui::Rect, window_size: egui::Vec2) {
        if rect.width() <= 1.0 || rect.height() <= 1.0 {
            return;
        }

        // --- THE FIX ---
        // Instead of checking the texture, we calculate the scale based on
        // the Raw Image vs The Window Size.
        // This handles High-DPI screens (e.g. Mac Retina or Windows at 150%) automatically.
        let scale_x = self.raw_image.width() as f32 / window_size.x;
        let scale_y = self.raw_image.height() as f32 / window_size.y;

        // Convert UI Coords -> Real Image Coords
        let x = (rect.min.x * scale_x) as u32;
        let y = (rect.min.y * scale_y) as u32;
        let width = (rect.width() * scale_x) as u32;
        let height = (rect.height() * scale_y) as u32;

        // --- The rest is exactly the same as before ---

        // Safety check: Ensure we don't crash if the user selects outside bounds
        if width == 0 || height == 0 { return; }

        let cropped_buffer = image::imageops::crop_imm(
            &self.raw_image,
            x.min(self.raw_image.width() - 1),
            y.min(self.raw_image.height() - 1),
            width.min(self.raw_image.width() - x),
            height.min(self.raw_image.height() - y)
        ).to_image();

        let pixels = cropped_buffer.into_raw();
        let image_data = ImageData {
            width: width as usize,
            height: height as usize,
            bytes: Cow::Owned(pixels),
        };

        if let Ok(mut clipboard) = Clipboard::new() {
            let _ = clipboard.set_image(image_data);
        }

        std::process::exit(0);
    }
}

impl eframe::App for CrabGrabApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. We prepare a variable to store our "Exit Action"
        //    We cannot execute it immediately because of borrow rules.
        ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        let mut finish_capture: Option<(egui::Rect, egui::Vec2)> = None;

        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {

            // --- Helper Closure ---
            // Captures &self.tiles (Immutable Borrow starts here)
            let draw_tiles = |painter: &egui::Painter, tint: egui::Color32| {
                for (offset_x, texture) in &self.tiles {
                    let tile_size = texture.size_vec2();
                    let rect = egui::Rect::from_min_size(
                        egui::Pos2::new(*offset_x, 0.0),
                        tile_size
                    );

                    painter.image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                        tint,
                    );
                }
            };

            // --- Draw Dark Layer ---
            draw_tiles(ui.painter(), egui::Color32::from_gray(120));

            // --- Handle Input ---
            let input = ctx.input(|i| i.clone());
            if input.pointer.any_pressed() {
                if let Some(pos) = input.pointer.interact_pos() {
                    self.start_pos = Some(pos);
                    self.current_pos = Some(pos);
                }
            } else if input.pointer.any_down() {
                if let Some(pos) = input.pointer.interact_pos() {
                    self.current_pos = Some(pos);
                }
            } else if input.pointer.any_released() {
                if let (Some(start), Some(end)) = (self.start_pos, self.current_pos) {
                    let rect = egui::Rect::from_two_pos(start, end);

                    // !!! FIX !!!
                    // Instead of calling self.handle_capture_finish(rect) here (which requires &mut self),
                    // we just SAVE the data to a local variable.
                    finish_capture = Some((rect, ui.max_rect().size()));
                }
            }

            // --- Draw Bright Layer ---
            if let (Some(start), Some(current)) = (self.start_pos, self.current_pos) {
                let selection_rect = egui::Rect::from_two_pos(start, current);

                let clip_painter = ui.painter().with_clip_rect(selection_rect);

                // We use 'draw_tiles' again here.
                // This is why we couldn't mutate self earlier!
                draw_tiles(&clip_painter, egui::Color32::WHITE);

                ui.painter().rect_stroke(
                    selection_rect,
                    0.0,
                    egui::Stroke::new(1.0, egui::Color32::WHITE),
                    eframe::epaint::StrokeKind::Middle,
                );

                ui.painter().rect_stroke(
                    selection_rect,
                    0.0,
                    egui::Stroke::new(1.0, egui::Color32::from_black_alpha(100)),
                    eframe::epaint::StrokeKind::Inside,
                );
            }
        }); // The scope ends here. 'draw_tiles' is dropped. The immutable borrow ends.

        // 2. NOW we are free to mutate self!
        if let Some((rect, window_size)) = finish_capture {
            self.handle_capture_finish(rect, window_size);
        }

        ctx.request_repaint();
    }
}