use std::borrow::Cow;
use std::time::Duration;
use arboard::{Clipboard, ImageData};
use eframe::egui;
use eframe::egui::vec2;
use global_hotkey::{GlobalHotKeyManager, GlobalHotKeyEvent, HotKeyState};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use image::{RgbaImage};
use tray_icon::menu::{MenuEvent, MenuId};
use tray_icon::{TrayIcon};
use std::sync::mpsc::{channel, Receiver};
#[allow(unused_imports)]
use rayon::prelude::*;

use crate::config::AppConfig;
use crate::utils;
use crate::audio::SoundEngine;

#[derive(Clone, Copy, Debug, PartialEq)]
enum AppState {
    Idle,
    Snapping,
    Config,
}

pub struct CrabGrabApp {
    state: AppState,
    previous_state: AppState,
    restore_rect: Option<egui::Rect>, // Stores position/size of settings window

    _hotkey_manager: GlobalHotKeyManager,
    cancel_hotkey: HotKey,
    settings_hotkey: HotKey,

    raw_image: Option<RgbaImage>,
    tiles: Option<Vec<(egui::Rect, egui::TextureHandle)>>,
    monitor_layout: Vec<egui::Rect>,
    start_pos: Option<egui::Pos2>,
    current_pos: Option<egui::Pos2>,
    virtual_origin: (f32, f32),
    physical_origin: (i32, i32),

    quit_id: MenuId,
    settings_id: MenuId,
    capture_id: MenuId,

    _tray_handle: Option<TrayIcon>,

    config: AppConfig,
    is_recording_hotkey: bool,
    file_picker_receiver: Option<Receiver<String>>,
    sound_engine: SoundEngine,
    cursor_texture: Option<egui::TextureHandle>,
}

impl CrabGrabApp {
    pub fn new(
        cc: &eframe::CreationContext,
        tray_handle: Option<TrayIcon>,
        quit_id: MenuId,
        settings_id: MenuId,
        capture_id: MenuId) -> Self {
        let loaded_config = AppConfig::load();

        let hotkey_manager = GlobalHotKeyManager::new().unwrap();
        let cancel_hotkey = HotKey::new(None, Code::Escape);
        let settings_hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyS);

        for hk in [loaded_config.snap_hotkey, cancel_hotkey, settings_hotkey] {
            match hotkey_manager.register(hk) {
                Ok(_) => log::info!("Hotkey registered: {:?}", hk),
                Err(e) => log::error!("Failed to register hotkey {:?}: {:?}", hk, e),
            }
        }

        let cursor_texture = {
            // 1. Load the bytes (Compile-time asset)
            // Make sure 'assets/cursor.png' exists!
            let image_data = include_bytes!("assets/cursor.png");

            // 2. Decode PNG
            if let Ok(image) = image::load_from_memory(image_data) {
                let size = [image.width() as usize, image.height() as usize];
                let image_buffer = image.to_rgba8();
                let pixels = image_buffer.as_flat_samples();

                // 3. Convert to egui::ColorImage
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    size,
                    pixels.as_slice(),
                );

                // 4. Upload to GPU
                // We use cc.egui_ctx here
                Some(cc.egui_ctx.load_texture(
                    "cursor_texture",
                    color_image,
                    egui::TextureOptions::NEAREST // Use NEAREST if it's pixel art!
                ))
            } else {
                log::error!("Failed to load cursor image");
                None
            }
        };

        let (virtual_origin, _) = if let Ok(data) = crate::capture::capture_all_screens() {
            log::info!("Warmup: Detected Origin at ({}, {}) with Scale {}",
            data.logical_origin.0, data.logical_origin.1, data.origin_scale_factor);

            // 2. Move the hidden window to that monitor immediately.
            // This forces Egui/Windows to handshake on the DPI (1.5) right now.
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                egui::pos2(data.logical_origin.0, data.logical_origin.1)
            ));

            // 3. Set a tiny non-zero size so the OS actually processes the move
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                egui::vec2(1.0, 1.0)
            ));

            (data.logical_origin, data.origin_scale_factor)
        } else {
            ((0.0, 0.0), 1.0)
        };

        Self {
            raw_image: None,
            tiles: None,
            monitor_layout: Vec::new(),
            start_pos: None,
            current_pos: None,
            state: AppState::Idle,
            _hotkey_manager: hotkey_manager,
            virtual_origin,
            physical_origin: (0, 0),
            cancel_hotkey,
            settings_hotkey,
            _tray_handle: tray_handle,
            quit_id,
            settings_id,
            capture_id,
            config: loaded_config,
            is_recording_hotkey: false,
            previous_state: AppState::Idle,
            restore_rect: None,
            file_picker_receiver: None,
            sound_engine: SoundEngine::new(),
            cursor_texture,
        }
    }

    fn handle_open_settings(&mut self, ctx: &egui::Context) {
        log::debug!("Opening Settings Window...");

        self.state = AppState::Config;

        // Apply window settings
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Transparent(false));

        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(600.0, 400.0)));
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(100.0, 100.0)));
    }

    fn handle_close_settings(&mut self, ctx: &egui::Context) {
        log::debug!("Closing Settings Window...");

        self.state = AppState::Idle;

        // Revert window settings
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Transparent(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(10000.0, 10000.0)));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(0.0, 0.0)));
        self.config.save();
    }

    /// Helper to handle system tray events (Right click menu, Left click toggle)
    fn handle_tray_events(&mut self, ctx: &egui::Context) {
        // 1. Drain Menu Events
        // (Menus don't usually spam, but it's good practice to limit them too)
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            log::debug!("MENU CLICK: {:?}", event.id);
            match event.id {
                _ if event.id == self.quit_id => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    self.config.save();
                },
                _ if event.id == self.settings_id => self.handle_open_settings(ctx),
                _ if event.id == self.capture_id => self.handle_begin_capture(ctx),
                _ => log::warn!("Warning: Unhandled Menu ID: {:?}", event.id),
            }
        }
    }

    fn handle_begin_capture(&mut self, ctx: &egui::Context) {
        // 1. Save where we came from
        self.previous_state = self.state;

        // 2. If coming from Config, save the window position/size
        if self.state == AppState::Config {
            // We grab the current outer rectangle of the window from egui context
            if let Some(rect) = ctx.input(|i| i.viewport().outer_rect) {
                self.restore_rect = Some(rect);
            }
        }

        log::debug!("Starting Capture from state: {:?}", self.previous_state);
        // 3. Prepare Window Style (Transparent Overlay)
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Transparent(true));

        if self.config.play_sound {
            self.sound_engine.play_activation();
        }

        match crate::capture::capture_all_screens() {
            Ok(data) => {
                self.raw_image = Some(data.full_image);
                self.virtual_origin = (0.0, 0.0);

                // CHANGED: Do NOT use ctx.pixels_per_point() here.
                // It is stale because the window hasn't moved yet.
                // Use the scale factor of the monitor where the window starts.
                let predicted_ppi = data.origin_scale_factor;

                log::debug!("Using Predicted PPI: {}", predicted_ppi);

                // 1. VISUALS: Pass Predicted PPI
                let tiles = utils::load_screens_as_tiles(
                    ctx,
                    &data.monitors,
                    data.physical_origin,
                    predicted_ppi // <--- Use the value from capture data
                );
                self.tiles = Some(tiles);

                // 2. HITBOXES: Pass Predicted PPI
                self.monitor_layout = data.monitors.iter().map(|m| {
                    let phys_offset_x = (m.x - data.physical_origin.0) as f32;
                    let phys_offset_y = (m.y - data.physical_origin.1) as f32;

                    // Divide by the predicted PPI
                    let egui_x = phys_offset_x / predicted_ppi;
                    let egui_y = phys_offset_y / predicted_ppi;

                    let egui_w = m.width as f32 / predicted_ppi;
                    let egui_h = m.height as f32 / predicted_ppi;

                    egui::Rect::from_min_size(
                        egui::pos2(egui_x, egui_y),
                        egui::vec2(egui_w, egui_h)
                    )
                }).collect();

                // ... Window positioning code remains the same ...
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                    egui::pos2(data.logical_origin.0, data.logical_origin.1)
                ));

                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                    egui::vec2(data.logical_width, data.logical_height)
                ));

                self.state = AppState::Snapping;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            Err(e) => log::error!("Capture failed: {}", e),
        }
    }

    fn handle_hotkey_events(&mut self, ctx: &egui::Context) {
        let receiver = GlobalHotKeyEvent::receiver();

        while let Ok(event) = receiver.try_recv() {
            if event.state == HotKeyState::Pressed {
                match event.id {
                    _ if event.id == self.config.snap_hotkey.id() => {
                        if matches!(self.state, AppState::Idle | AppState::Config) {
                            self.handle_begin_capture(ctx);
                        }
                    }
                    _ if event.id == self.cancel_hotkey.id() => {
                        if matches!(self.state, AppState::Snapping) {
                            self.state = AppState::Idle;
                            self.start_pos = None;
                            self.current_pos = None;
                            self.raw_image = None;
                            self.tiles = None;
                            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(10000.0, 10000.0)));
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(0.0, 0.0)));
                        }
                    }
                    _ if event.id == self.settings_hotkey.id() => {
                        if !matches!(self.state, AppState::Config) {
                            self.handle_open_settings(ctx);
                        } else {
                            self.handle_close_settings(ctx);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_capture_finish(&mut self, ctx: &egui::Context, rect: egui::Rect, window_size: egui::Vec2) {
        if rect.width() <= 1.0 || rect.height() <= 1.0 {
            return;
        }

        // 1. CROP (Must be done on Main Thread to access self.raw_image)
        // We clone the cropped buffer so the background thread can own it.
        let cropped_buffer = if let Some(image) = &self.raw_image {
            let scale_x = image.width() as f32 / window_size.x;
            let scale_y = image.height() as f32 / window_size.y;

            let x = (rect.min.x * scale_x) as u32;
            let y = (rect.min.y * scale_y) as u32;
            let width = (rect.width() * scale_x) as u32;
            let height = (rect.height() * scale_y) as u32;

            image::imageops::crop_imm(
                image,
                x.min(image.width() - 1),
                y.min(image.height() - 1),
                width.min(image.width() - x),
                height.min(image.height() - y)
            ).to_image()
        } else {
            return;
        };

        if self.config.play_sound {
            self.sound_engine.play_shutter();
        }

        // 2. PREPARE DATA FOR BACKGROUND THREAD
        // We need to clone small config strings to move them into the thread.
        let save_path = self.config.save_directory.clone();
        let auto_save = self.config.auto_save;

        // 3. SPAWN BACKGROUND TASK (Fire and Forget)
        // Rayon uses a thread pool, so this is very efficient.
        rayon::spawn(move || {
            // A. Save to Disk (The Slow Part)
            if auto_save {
                utils::save_image_to_disk(&cropped_buffer, &save_path);
            }

            // B. Copy to Clipboard
            // Converting to raw bytes takes a little time too, so we do it here.
            let width = cropped_buffer.width();
            let height = cropped_buffer.height();
            let pixels = cropped_buffer.into_raw();

            let image_data = ImageData {
                width: width as usize,
                height: height as usize,
                bytes: Cow::Owned(pixels),
            };

            if let Ok(mut clipboard) = Clipboard::new() {
                if let Err(e) = clipboard.set_image(image_data) {
                    log::error!("Failed to copy to clipboard: {}", e);
                } else {
                    log::debug!("Copied to clipboard successfully.");
                }
            }
        });

        // 4. INSTANT UI RESTORE
        // We don't wait for the save/clipboard. We hide the window immediately.
        log::debug!("Capture Finished. Restoring to: {:?}", self.previous_state);

        match self.previous_state {
            AppState::Config => {
                self.state = AppState::Config;
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Transparent(false));

                if let Some(saved_rect) = self.restore_rect {
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(saved_rect.min));
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(saved_rect.size()));
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(100.0, 100.0)));
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(600.0, 400.0)));
                }
            },
            _ => {
                self.state = AppState::Idle;
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Transparent(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(10000.0, 10000.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(0.0, 0.0)));
            }
        }

        // --- CLEANUP ---
        self.raw_image = None;
        self.tiles = None;
        self.restore_rect = None;
        self.start_pos = None;
        self.current_pos = None;
    }

    fn convert_egui_to_hotkey(&self, _egui_key: egui::Key, modifiers: egui::Modifiers) -> Option<HotKey> {
        // 1. Convert egui::Modifiers -> global_hotkey::hotkey::Modifiers
        let mut gh_modifiers = Modifiers::empty();

        if modifiers.ctrl { gh_modifiers |= Modifiers::CONTROL; }
        if modifiers.shift { gh_modifiers |= Modifiers::SHIFT; }
        if modifiers.alt { gh_modifiers |= Modifiers::ALT; }

        // 2. Convert egui::Key -> global_hotkey::hotkey::Code
        let gh_code = {
            macro_rules! map_letters {
                ( $( $egui:ident => $gh:ident ),* $(,)? ) => {
                    match _egui_key {
                        $( egui::Key::$egui => Code::$gh, )*
                        _ => {
                            log::warn!("Unsupported key: {:?}", _egui_key);
                            return None;
                        }
                    }
                };
            }

            map_letters!(
                A => KeyA, B => KeyB, C => KeyC, D => KeyD, E => KeyE, F => KeyF,
                G => KeyG, H => KeyH, I => KeyI, J => KeyJ, K => KeyK, L => KeyL,
                M => KeyM, N => KeyN, O => KeyO, P => KeyP, Q => KeyQ, R => KeyR,
                S => KeyS, T => KeyT, U => KeyU, V => KeyV, W => KeyW, X => KeyX,
                Y => KeyY, Z => KeyZ,
                Num0 => Digit0, Num1 => Digit1, Num2 => Digit2, Num3 => Digit3,
                Num4 => Digit4, Num5 => Digit5, Num6 => Digit6, Num7 => Digit7,
                Num8 => Digit8, Num9 => Digit9
            )
        };

        Some(HotKey::new(Some(gh_modifiers), gh_code))
    }

    fn update_hotkey(&mut self, new_hotkey: HotKey) {
        log::debug!("Updating hotkey to: {:?}", new_hotkey);

        // 1. Unregister the OLD hotkey (self.config.snap_hotkey)
        let result = self._hotkey_manager.unregister(self.config.snap_hotkey);
        // Hint: self.hotkey_manager.unregister(self.config.snap_hotkey)

        if let Err(e) = result {
            log::error!("Failed to unregister old hotkey {:?}: {:?}", self.config.snap_hotkey, e);
            return;
        }

        // 2. Register the NEW hotkey
        // Hint: self.hotkey_manager.register(new_hotkey)
        let result = self._hotkey_manager.register(new_hotkey);
        if let Err(e) = result {
            log::error!("Failed to register new hotkey {:?}: {:?}", new_hotkey, e);
            // Attempt to restore the previous hotkey; log any failure but don't panic.
            if let Err(e2) = self._hotkey_manager.register(self.config.snap_hotkey) {
                log::error!("Failed to restore previous hotkey {:?}: {:?}", self.config.snap_hotkey, e2);
            }
            return;
        }

        // 4. Update the config state
        self.config.snap_hotkey = new_hotkey;
    }

    fn open_file_picker(&mut self) {
        log::debug!("Spawning file picker thread...");
        // TASK: Spawn a thread to pick a folder.
        // 1. Create a channel (tx, rx).
        let (tx, rx) = channel();
        // 2. Store 'rx' in self.file_picker_receiver.
        self.file_picker_receiver = Some(rx);
        // 3. Spawn a std::thread.
        std::thread::spawn(move || {
            // 4. Inside the thread: call rfd::FileDialog::new().pick_folder().
            if let Some(path_buf) = rfd::FileDialog::new().pick_folder() {
                // 5. If a path is found, convert to String and send it via 'tx'.
                if let Some(path_str) = path_buf.to_str() {
                    let _ = tx.send(path_str.to_string());
                }
            }
        });
    }

    fn check_file_picker_result(&mut self) {
        if let Some(rx) = &self.file_picker_receiver {
            match rx.try_recv() {
                Ok(new_path) => {
                    log::debug!("File picker returned path: {}", new_path);
                    self.config.save_directory = new_path;
                    self.file_picker_receiver = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(e) => {
                    log::error!("File picker channel error: {:?}", e);
                    self.file_picker_receiver = None;
                }
            }
        }
    }
}

impl eframe::App for CrabGrabApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_tray_events(ctx);
        self.handle_hotkey_events(ctx);
        self.check_file_picker_result();

        // --- Drawing Logic ---
        match self.state {
            AppState::Idle => {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(10000.0, 10000.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(0.0, 0.0)));
                ctx.request_repaint_after(Duration::from_millis(100));
            }
            AppState::Snapping => {
                let mut finish_capture: Option<(egui::Rect, egui::Vec2)> = None;

                egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
                    let draw_tiles = |painter: &egui::Painter, tint: egui::Color32| {
                        if let Some(tiles) = &self.tiles {
                            for (rect, texture) in tiles {
                                painter.image(
                                    texture.id(),
                                    *rect, // Rect is already in local physical coords (0,0 based)
                                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                                    tint,
                                );
                            }
                        }
                    };

                    // 1. Background (Dark)
                    draw_tiles(ui.painter(), egui::Color32::from_gray(120));

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
                    }  else if input.pointer.any_released() {
                        if let (Some(start), Some(end)) = (self.start_pos, self.current_pos) {
                            let rect = egui::Rect::from_two_pos(start, end);
                            finish_capture = Some((rect, ui.max_rect().size()));
                        }
                    }

                    if let (Some(start), Some(current)) = (self.start_pos, self.current_pos) {
                        let selection_rect = egui::Rect::from_two_pos(start, current);
                        let clip_painter = ui.painter().with_clip_rect(selection_rect);

                        // Draw the tiles inside the selection with FULL brightness (No tint)
                        draw_tiles(&clip_painter, egui::Color32::WHITE);

                        // CHANGE: Use BLACK stroke so it stands out against the white background
                        ui.painter().rect_stroke(
                            selection_rect,
                            0.0,
                            egui::Stroke::new(2.0, egui::Color32::BLACK),
                            eframe::epaint::StrokeKind::Middle,
                        );

                        // Optional: Inner white line for "marching ants" contrast
                        ui.painter().rect_stroke(
                            selection_rect,
                            0.0,
                            egui::Stroke::new(1.0, egui::Color32::WHITE),
                            eframe::epaint::StrokeKind::Inside,
                        );
                    }

                    // 2. Foreground (Bright)
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

                    if self.config.custom_cursor {
                        if let Some(texture) = &self.cursor_texture {
                            ctx.set_cursor_icon(egui::CursorIcon::None);
                            utils::draw_custom_cursor(ui, texture);
                        } else {
                            // Fallback if texture failed to load
                            ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
                        }
                    } else {
                        ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
                    }
                });

                if let Some((rect, window_size)) = finish_capture {
                    self.handle_capture_finish(ctx, rect, window_size);
                }


            }
            AppState::Config => {
                // 1. Handle "X" Button (Close Request)
                // If user clicked X on the window title bar:
                if ctx.input(|i| i.viewport().close_requested()) {
                    // A. Cancel the actual kill command
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    self.handle_close_settings(ctx);
                }

                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("CrabGrab Settings");
                    ui.separator();

                    // 1. Storage & Saving
                    ui.heading("Storage");
                    ui.horizontal(|ui| {
                        ui.label("Save Location:");
                        // Display the path in a monospace font so it looks like code
                        ui.code(&self.config.save_directory);

                        if ui.button("📂 Browse...").clicked() {
                            self.open_file_picker();
                        }
                    });

                    ui.checkbox(&mut self.config.auto_save, "Auto-save screenshots to file");

                    ui.separator();

                    // 2. Visuals & Audio
                    ui.heading("Experience");
                    ui.checkbox(&mut self.config.custom_cursor, "Use CrabGrab Cursor");
                    ui.checkbox(&mut self.config.play_sound, "Play Camera Shutter Sound");

                    if ui.checkbox(&mut self.config.run_on_startup, "Run on Startup").changed() {
                        utils::set_autostart(self.config.run_on_startup);
                        self.config.save();
                    }

                    ui.separator();

                    // 3. Shortcuts
                    ui.heading("Shortcuts");
                    ui.horizontal(|ui| {
                        ui.label("Capture Screen:");

                        let btn_text = if self.is_recording_hotkey {
                            "Press any key... (Esc to cancel)".to_string()
                        } else {
                            // FIX: Use the new utility function
                            utils::format_hotkey(&self.config.snap_hotkey)
                        };

                        let btn = ui.button(btn_text);
                        if btn.clicked() {
                            self.is_recording_hotkey = true;
                        }

                        if self.is_recording_hotkey {
                            ui.memory_mut(|m| m.request_focus(btn.id));
                            let input = ctx.input(|i| i.clone());

                            if input.key_pressed(egui::Key::Escape) {
                                self.is_recording_hotkey = false;
                            }

                            for key in input.keys_down {
                                if let Some(new_hotkey) = self.convert_egui_to_hotkey(key, input.modifiers) {
                                    self.update_hotkey(new_hotkey);
                                    self.is_recording_hotkey = false;
                                    break;
                                }
                            }
                        }
                    });

                    ui.add_space(20.0);

                    // Bottom Action Bar
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        if ui.button("Close Settings").clicked() {
                            self.handle_close_settings(ctx);
                        }
                    });
                });
            }
        }
    }
}

