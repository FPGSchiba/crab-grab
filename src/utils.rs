use std::env;
use std::path::Path;
use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions};
use egui::{vec2};
use global_hotkey::hotkey::{HotKey, Modifiers};
use image::RgbaImage;
use tray_icon::Icon;
use auto_launch::AutoLaunchBuilder;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

const MAX_TEXTURE_SIZE: u32 = 2048;

/// Splits a large image into vertical tiles that fit within GPU limits (max 2048px width).
/// Returns a vector of (X_Offset, TextureHandle).
pub fn load_image_as_tiles(ctx: &Context, image: &RgbaImage) -> Vec<(f32, TextureHandle)> {
    let (total_width, height) = image.dimensions();

    let mut tiles = Vec::new();
    let mut current_x = 0;

    while current_x < total_width {
        // Determine the width of this specific tile
        let tile_width = std::cmp::min(MAX_TEXTURE_SIZE, total_width - current_x);

        // Crop the strip from the original image
        let sub_image = image::imageops::crop_imm(
            image,
            current_x,
            0,
            tile_width,
            height
        ).to_image();

        // Convert to egui ColorImage
        let pixels = sub_image.as_flat_samples();
        let color_image = ColorImage::from_rgba_unmultiplied(
            [tile_width as usize, height as usize],
            pixels.as_slice(),
        );

        // Upload to GPU
        let name = format!("screenshot_tile_{}", current_x);
        let handle = ctx.load_texture(&name, color_image, TextureOptions::NEAREST);

        // Store the X offset so we know where to draw it later
        tiles.push((current_x as f32, handle));

        current_x += tile_width;
    }

    tiles
}

/// Takes multiple images (with X, Y positions) and turns them into GPU tiles.
/// Returns: Vec<(Global_X_Position, Global_Y_Position, TextureHandle)>
pub fn load_screens_as_tiles(
    ctx: &Context,
    captures: &[(i32, i32, RgbaImage)]
) -> Vec<(f32, f32, TextureHandle)> {
    // 1. Loop through each image.
    let mut result_tiles: Vec<(f32, f32, TextureHandle)> = Vec::new();
    for (img_x, img_y, image) in captures {
        // 2. For each image, perform the tiling logic (splitting into 2048px strips).
        let tiles = load_image_as_tiles(ctx, image);

        // For each tile, we need to adjust its global position.
        for (tile_offset_x, texture) in tiles {
            let global_x = *img_x as f32 + tile_offset_x;
            let global_y = *img_y as f32;

            // Store the global position and texture handle.
            // Note: You would typically collect these into a result vector.
            // 3. IMPORTANT: The tile's X position is: Image_X + Current_Strip_Offset.
            result_tiles.push((global_x, global_y, texture.clone()));
        }
    }

    // 4. Return one big flat list of all tiles from all monitors.
    result_tiles
}

/// Helper to load an icon from a file path or bytes.
/// Hint: Use `image::open` or `image::load_from_memory`.
/// Key Step: You must convert the image to RGBA8 (4 bytes per pixel).
pub fn load_tray_icon() -> Icon {
    // 1. Load image (e.g., "assets/icon.png" or a generic one for now)
    let logo = include_bytes!("assets/logo.png");
    // 2. Get width, height, and raw rgba vectors.
    let img = image::load_from_memory(logo).expect("Failed to load icon image");
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();
    let rgba = rgba_img.into_raw();
    // 3. Return Icon::from_rgba(rgba, width, height).unwrap()
    Icon::from_rgba(rgba, width, height).unwrap()
}

pub fn format_hotkey(hotkey: &HotKey) -> String {
    let mut text = String::new();
    let mods = hotkey.mods;

    if mods.contains(Modifiers::CONTROL) { text.push_str("Ctrl + "); }
    if mods.contains(Modifiers::SHIFT)   { text.push_str("Shift + "); }
    if mods.contains(Modifiers::ALT)     { text.push_str("Alt + "); }
    if mods.contains(Modifiers::META)    { text.push_str("Win + "); }

    // Clean up the Code string (e.g. "KeyG" -> "G")
    let key_str = format!("{:?}", hotkey.key);
    let clean_key = key_str.strip_prefix("Key").unwrap_or(&key_str);

    text.push_str(clean_key);
    text
}

pub fn save_image_to_disk(image: &RgbaImage, dir_path: &str) {
    let time_now = chrono::Local::now();
    let timestamp = time_now.format("%Y-%m-%d_%H-%M-%S").to_string();
    let path = Path::new(dir_path).join(format!("screenshot_{}.png", timestamp));
    log::info!("Saving image to: {}", dir_path);
    if let Err(e) = std::fs::create_dir_all(dir_path) {
        log::error!("Failed to create directory {}: {}", dir_path, e);
        return;
    }
    match image.save(&path) {
        Ok(_) => log::info!("Image saved successfully to {:?}", path),
        Err(e) => log::error!("Failed to save image to {:?}: {}", path, e),
    }
}

pub fn draw_custom_cursor(ui: &mut egui::Ui, texture: &egui::TextureHandle) {
    let pointer_pos = match ui.input(|i| i.pointer.latest_pos()) {
        Some(pos) => pos,
        None => return,
    };

    let painter = ui.ctx().layer_painter(eframe::egui::LayerId::new(
        eframe::egui::Order::Tooltip,
        eframe::egui::Id::new("cursor_overlay")
    ));

    // Size: You can hardcode this (e.g., 32.0) or use the image size
    let size = vec2(32.0, 32.0);

    // Offset: We want the "Tip" of the claw to be at the mouse pointer.
    // If your image has the tip at the top-left (0,0), this is simple:
    let rect = egui::Rect::from_min_size(pointer_pos, size);

    // Draw the image
    painter.image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)), // UV coords (0-1)
        egui::Color32::WHITE,
    );
}

pub fn set_autostart(enable: bool) {
    // Get the absolute path to the current executable
    if let Ok(current_exe) = env::current_exe() {
        let current_exe_str = current_exe.to_str().unwrap();

        // Initialize the AutoLaunch handler
        // 'app_name' should be unique to your app
        let auto = AutoLaunchBuilder::new()
            .set_app_name("CrabGrab")
            .set_app_path(current_exe_str)
            .set_use_launch_agent(true) // For macOS
            .build();

        if let Ok(auto) = auto {
            if enable {
                if auto.is_enabled().unwrap_or(false) { return; }
                let _ = auto.enable();
                log::debug!("Autostart ENABLED");
            } else {
                if !auto.is_enabled().unwrap_or(false) { return; }
                let _ = auto.disable();
                log::debug!("Autostart DISABLED");
            }
        }
    }
}

pub fn get_logging_config() -> Config {
    let log_file_path = dirs::config_dir().unwrap().join("crab-grab").join("crab-grab.log");

    // Define a console appender
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {l} - {m}\n")))
        .build();

    let file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {l} - {m}\n")))
        .build(log_file_path)
        .unwrap();

    // Build the logging configuration
    Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(file)))
        .build(
            Root::builder()
                .appender("stdout")
                .appender("file")
                .build(log::LevelFilter::Info),
        )
        .unwrap()
}
