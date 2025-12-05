use image::RgbaImage;
use std::error::Error;

/// Finds the primary monitor and returns its pixels.
/// Returns a generic Result because screen capture can fail (permissions, etc).
pub fn take_screenshot() -> Result<RgbaImage, Box<dyn Error>> {
    // 1. Get all monitors using xcap::Monitor::all()
    let monitors = xcap::Monitor::all()?;
    // 2. Find the primary monitor (or just take the first one for now)
    let monitor = monitors
        .into_iter()
        .find(|m| m.is_primary().unwrap())
        .ok_or("No monitors found")?;
    // 3. Capture the image using monitor.capture_image()
    let captured_image = monitor.capture_image()?;
    // 4. Convert the result to an RgbaImage and return it
    Ok(captured_image)
}