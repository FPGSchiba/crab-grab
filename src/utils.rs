// src/utils.rs
use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions};
use image::RgbaImage;

/// Splits a large image into vertical tiles that fit within GPU limits (max 2048px width).
/// Returns a vector of (X_Offset, TextureHandle).
pub fn load_image_as_tiles(ctx: &Context, image: &RgbaImage) -> Vec<(f32, TextureHandle)> {
    let max_texture_side = 2048; // Safe limit for almost all GPUs
    let (total_width, height) = image.dimensions();

    let mut tiles = Vec::new();
    let mut current_x = 0;

    while current_x < total_width {
        // Determine the width of this specific tile
        let tile_width = std::cmp::min(max_texture_side, total_width - current_x);

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