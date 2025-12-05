use eframe::{egui, NativeOptions, Renderer};
// Update imports to include WgpuSetup and WgpuSetupCreateNew
use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew, wgpu};
use std::sync::Arc;

mod app;
mod capture;
mod utils;

fn main() -> Result<(), eframe::Error> {
    // 1. Take the screenshot
    let screenshot = match capture::take_screenshot() {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Failed to capture screen: {}", e);
            std::process::exit(1);
        }
    };

    // 2. Configure WGPU (Updated for eframe 0.33+)
    // "device_descriptor" is now nested inside "wgpu_setup"
    let wgpu_options = WgpuConfiguration {
        wgpu_setup: WgpuSetup::CreateNew(WgpuSetupCreateNew {
            device_descriptor: Arc::new(|_adapter| {
                let mut limits = wgpu::Limits::default();
                // Increase texture size limit to 8192px (supports 8K screens)
                limits.max_texture_dimension_2d = 8192;

                wgpu::DeviceDescriptor {
                    label: Some("CrabGrab"),
                    required_features: wgpu::Features::default(),
                    required_limits: limits,
                    experimental_features: Default::default(),
                    memory_hints: Default::default(),
                    trace: Default::default(),
                }
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    // 3. Set up the window options
    let native_options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_fullscreen(true)
            .with_always_on_top()
            .with_transparent(true),

        renderer: Renderer::Wgpu,
        wgpu_options, // Pass the corrected options here

        ..Default::default()
    };

    // 4. Run the app
    eframe::run_native(
        "Crab Grab",
        native_options,
        Box::new(|cc| Ok(Box::new(app::CrabGrabApp::new(cc, screenshot)))),
    )
}