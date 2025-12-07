use xcap::Monitor;
use image::RgbaImage;
use std::error::Error;

#[derive(Clone, Debug)]
pub struct MonitorData {
    pub x: i32,      // Physical X
    pub y: i32,      // Physical Y
    pub width: u32,  // Physical Width
    pub height: u32, // Physical Height
    pub scale_factor: f32,
    pub image: RgbaImage,
}

pub struct CaptureData {
    pub monitors: Vec<MonitorData>,
    pub full_image: RgbaImage,

    // We need both Origins.
    // 1. Logical: To tell the OS where to put the Window.
    pub logical_origin: (f32, f32),
    pub logical_width: f32,
    pub logical_height: f32,

    pub origin_scale_factor: f32,

    // 2. Physical: To tell Egui where to draw the pixels inside the window.
    pub physical_origin: (i32, i32),
    pub physical_width: u32,
    pub physical_height: u32,
}

pub fn capture_all_screens() -> Result<CaptureData, Box<dyn Error>> {
    let monitors = Monitor::all()?;
    if monitors.is_empty() { return Err("No monitors found".into()); }

    log::debug!("--- CAPTURE DEBUG START ---");

    let captures: Vec<MonitorData> = monitors.into_iter().enumerate().map(|(i, monitor)| {
        let scale = monitor.scale_factor().unwrap_or(1.0);
        let phys_x = monitor.x()?;
        let phys_y = monitor.y()?;
        let phys_w = monitor.width()?;
        let phys_h = monitor.height()?;

        log::debug!("Monitor #{}: PhysRect=[x:{}, y:{}, w:{}, h:{}], Scale={}",
            i, phys_x, phys_y, phys_w, phys_h, scale);

        let image = monitor.capture_image()?;

        Ok(MonitorData {
            x: phys_x, y: phys_y, width: phys_w, height: phys_h,
            scale_factor: scale, image
        })
    }).collect::<Result<Vec<MonitorData>, Box<dyn Error>>>()?;

    // --- 1. CALCULATE PHYSICAL BOUNDS (For internal drawing) ---
    let mut min_phys_x = i32::MAX;
    let mut min_phys_y = i32::MAX;
    let mut max_phys_x = i32::MIN;
    let mut max_phys_y = i32::MIN;

    for mon in &captures {
        min_phys_x = min_phys_x.min(mon.x);
        min_phys_y = min_phys_y.min(mon.y);
        max_phys_x = max_phys_x.max(mon.x + mon.width as i32);
        max_phys_y = max_phys_y.max(mon.y + mon.height as i32);
    }

    let mut origin_scale_factor = 1.0;
    for mon in &captures {
        if mon.x == min_phys_x && mon.y == min_phys_y {
            origin_scale_factor = mon.scale_factor;
            break;
        }
    }

    let total_phys_w = (max_phys_x - min_phys_x) as u32;
    let total_phys_h = (max_phys_y - min_phys_y) as u32;

    log::debug!("Bounds Physical: Origin=({}, {}), Size={}x{}",
        min_phys_x, min_phys_y, total_phys_w, total_phys_h);

    // --- 2. CALCULATE LOGICAL BOUNDS (For OS Window positioning) ---
    // We must respect the scale factor for the Window Manager
    let mut min_log_x = f32::MAX;
    let mut min_log_y = f32::MAX;
    let mut max_log_x = f32::MIN;
    let mut max_log_y = f32::MIN;

    for (i, mon) in captures.iter().enumerate() {
        let log_x = mon.x as f32 / mon.scale_factor;
        let log_y = mon.y as f32 / mon.scale_factor;
        let log_w = mon.width as f32 / mon.scale_factor;
        let log_h = mon.height as f32 / mon.scale_factor;

        log::debug!("Mon #{}: PhysX={} / Scale {:.2} = LogX {:.2}", i, mon.x, mon.scale_factor, log_x);
        log::debug!("Mon #{}: PhysW={} / Scale {:.2} = LogW {:.2}", i, mon.width, mon.scale_factor, log_w);

        min_log_x = min_log_x.min(log_x);
        min_log_y = min_log_y.min(log_y);
        max_log_x = max_log_x.max(log_x + log_w);
        max_log_y = max_log_y.max(log_y + log_h);
    }

    log::debug!("Bounds Logical: Origin=({}, {}), Size={}x{}",
        min_log_x, min_log_y, max_log_x - min_log_x, max_log_y - min_log_y);

    // --- 3. STITCH FULL IMAGE ---
    let mut full_image = RgbaImage::new(total_phys_w, total_phys_h);
    for mon in &captures {
        // Normalize: Screen X - Leftmost X = Local X
        let local_x = (mon.x - min_phys_x) as i64;
        let local_y = (mon.y - min_phys_y) as i64;

        image::imageops::overlay(
            &mut full_image,
            &mon.image,
            local_x,
            local_y
        );
    }

    Ok(CaptureData {
        monitors: captures,
        full_image,
        logical_origin: (min_log_x, min_log_y),
        logical_width: max_log_x - min_log_x,
        logical_height: max_log_y - min_log_y,
        origin_scale_factor,
        physical_origin: (min_phys_x, min_phys_y),
        physical_width: total_phys_w,
        physical_height: total_phys_h,
    })
}