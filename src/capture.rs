use xcap::Monitor;
use image::RgbaImage;
use std::error::Error;

// 1. The Struct needed for the App logic
pub struct MonitorData {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub image: RgbaImage,
}

pub struct CaptureData {
    // 2. We store the rich data here instead of the tuple
    pub monitors: Vec<MonitorData>,
    pub full_image: RgbaImage,
    pub origin: (i32, i32),
}

pub fn capture_all_screens() -> Result<CaptureData, Box<dyn Error>> {
    let monitors = Monitor::all()?;
    if monitors.is_empty() { return Err("No monitors found".into()); }

    // --- CAPTURE LOOP ---
    // We map monitors directly to our MonitorData struct
    let captures: Vec<MonitorData> = monitors.into_iter().map(|monitor| {
        // Retrieve dimensions explicitly
        let x = monitor.x()?;
        let y = monitor.y()?;
        let width = monitor.width()?;
        let height = monitor.height()?;
        let image = monitor.capture_image()?;

        Ok(MonitorData {
            x,
            y,
            width,
            height,
            image
        })
    }).collect::<Result<Vec<MonitorData>, Box<dyn Error>>>()?;

    // --- CALCULATE BOUNDS ---
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for mon in &captures {
        min_x = min_x.min(mon.x);
        min_y = min_y.min(mon.y);
        // We use the width/height from the struct now
        max_x = max_x.max(mon.x + mon.width as i32);
        max_y = max_y.max(mon.y + mon.height as i32);
    }

    // --- STITCHING ---
    let total_w = (max_x - min_x) as u32;
    let total_h = (max_y - min_y) as u32;
    let mut full_image = RgbaImage::new(total_w, total_h);

    for mon in &captures {
        // Overlay puts the monitor image onto the canvas
        image::imageops::overlay(
            &mut full_image,
            &mon.image,
            (mon.x - min_x) as i64,
            (mon.y - min_y) as i64
        );
    }

    Ok(CaptureData {
        monitors: captures, // Pass the rich data back
        full_image,
        origin: (min_x, min_y),
    })
}