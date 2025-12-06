use image::{GenericImage, RgbaImage};
use xcap::Monitor;
use std::error::Error;
#[allow(unused_imports)]
use rayon::prelude::*;

pub struct CaptureData {
    pub screenshots: Vec<(i32, i32, RgbaImage)>,
    pub full_image: RgbaImage,
    pub origin: (i32, i32),
}

pub fn capture_all_screens() -> Result<CaptureData, Box<dyn Error>> {
    let monitors = Monitor::all()?;
    if monitors.is_empty() { return Err("No monitors found".into()); }

    // --- PARALLEL CAPTURE START ---
    // Instead of a for-loop, we use par_iter to capture all screens at the same time.
    let captures_result: Result<Vec<(i32, i32, RgbaImage)>, Box<dyn Error>> = monitors.iter().map(|monitor| {
        let x = monitor.x()?;
        let y = monitor.y()?;
        // This is the slow part, now running on multiple threads!
        let img = monitor.capture_image()?;
        Ok((x, y, img))
    }).collect();

    let screenshots = captures_result?;
    // --- PARALLEL CAPTURE END ---

    // Calculate bounds (Same as before)
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for (x, y, img) in &screenshots {
        min_x = min_x.min(*x);
        min_y = min_y.min(*y);
        max_x = max_x.max(x + img.width() as i32);
        max_y = max_y.max(y + img.height() as i32);
    }

    // Stitching (This is fast enough in memory to keep sequential usually)
    let total_w = (max_x - min_x) as u32;
    let total_h = (max_y - min_y) as u32;
    let mut full_image = RgbaImage::new(total_w, total_h);

    for (x, y, img) in &screenshots {
        full_image.copy_from(img, (*x - min_x) as u32, (*y - min_y) as u32)?;
    }

    Ok(CaptureData {
        screenshots,
        full_image,
        origin: (min_x, min_y),
    })
}