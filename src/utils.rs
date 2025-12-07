use std::env;
use std::path::Path;
use eframe::egui::{Context, TextureHandle, TextureOptions};
use egui::{vec2};
use global_hotkey::hotkey::{HotKey, Modifiers};
use image::RgbaImage;
use tray_icon::Icon;
use auto_launch::AutoLaunchBuilder;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use crate::capture::{MonitorData};

const MAX_TILE_SIZE: u32 = 2048; // Safe limit for almost any GPU

pub fn load_image_as_tiles(ctx: &Context, image: &RgbaImage) -> Vec<(f32, f32, TextureHandle)> {
    let (total_width, total_height) = image.dimensions();
    let mut tiles = Vec::new();

    let mut current_y = 0;
    while current_y < total_height {
        let tile_height = std::cmp::min(MAX_TILE_SIZE, total_height - current_y);

        let mut current_x = 0;
        while current_x < total_width {
            let tile_width = std::cmp::min(MAX_TILE_SIZE, total_width - current_x);

            // Crop the specific rectangle (Grid cell)
            let sub_image = image::imageops::crop_imm(
                image,
                current_x,
                current_y,
                tile_width,
                tile_height
            ).to_image();

            let pixels = sub_image.as_flat_samples();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [tile_width as usize, tile_height as usize],
                pixels.as_slice(),
            );

            // Unique name for caching
            let name = format!("tile_{}_{}_{}x{}", current_x, current_y, tile_width, tile_height);
            let handle = ctx.load_texture(&name, color_image, TextureOptions::NEAREST);

            // Store X AND Y offsets
            tiles.push((current_x as f32, current_y as f32, handle));

            current_x += tile_width;
        }
        current_y += tile_height;
    }

    tiles
}

pub fn load_screens_as_tiles(
    ctx: &egui::Context,
    captures: &[MonitorData]
) -> Vec<(f32, f32, egui::TextureHandle)> {
    let mut result_tiles = Vec::new();

    for mon in captures {
        // 1. Tile this specific monitor's image
        let local_tiles = load_image_as_tiles(ctx, &mon.image);

        // 2. Adjust offsets to Global Space
        for (tile_x, tile_y, texture) in local_tiles {
            // Global X = Monitor X + Tile X Offset
            let global_x = mon.x as f32 + tile_x;
            // Global Y = Monitor Y + Tile Y Offset
            let global_y = mon.y as f32 + tile_y;

            result_tiles.push((global_x, global_y, texture));
        }
    }

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

pub fn setup_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        log::error!("CRASH: App panicked!");
        if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            log::error!("Panic Payload: {:?}", s);
        }
        if let Some(location) = panic_info.location() {
            log::error!("Location: {}:{}:{}", location.file(), location.line(), location.column());
        }
        // Call default hook to print to stderr (if console exists)
        default_hook(panic_info);
    }));
}
