use eframe::{egui, NativeOptions, Renderer};
use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew, wgpu};
use std::sync::Arc;
use tray_icon::{TrayIcon, TrayIconBuilder, menu::{Menu, MenuItem, MenuId}};

mod app;
mod capture;
mod utils;
mod config;
mod audio;

// --- WINDOWS SPECIFIC IMPORTS ---
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, TranslateMessage, DispatchMessageW, MSG};

fn main() -> Result<(), eframe::Error> {
    // 1. Setup Common Menu Items
    let quit_id = "quit".to_string();
    let settings_id = "settings".to_string();
    let capture_id = "capture".to_string();

    // 2. Initialize Tray (Platform Dependent Logic)
    // We get back an Option<TrayIcon>.
    // On Windows, this is None (because the icon lives in a thread).
    // On Mac/Linux, this is Some(icon) (because we must keep it alive in the App).
    let _tray_handle = init_tray_platform(
        quit_id.clone(),
        settings_id.clone(),
        capture_id.clone(),
    );

    // 3. WGPU Setup
    let wgpu_options = WgpuConfiguration {
        wgpu_setup: WgpuSetup::CreateNew(WgpuSetupCreateNew {
            device_descriptor: Arc::new(|_adapter| {
                let mut limits = wgpu::Limits::default();
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

    let native_options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_always_on_top()
            .with_taskbar(false)
            .with_transparent(true)
            .with_position(egui::pos2(10000.0, 10000.0)),
        renderer: Renderer::Wgpu,
        wgpu_options,
        ..Default::default()
    };

    eframe::run_native(
        "Crab Grab",
        native_options,
        Box::new(move |cc| {
            // We pass the handle (if it exists) into the app to keep it alive
            Ok(Box::new(app::CrabGrabApp::new(cc, _tray_handle, MenuId::new(quit_id), MenuId::new(settings_id), MenuId::new(capture_id))))
        }),
    )
}

// ---------------------------------------------------------
// CROSS PLATFORM TRAY LOGIC
// ---------------------------------------------------------

/// Windows: Spawns thread. Creates Items INSIDE the thread.
#[cfg(target_os = "windows")]
fn init_tray_platform(quit_id: String, settings_id: String, capture_id: String) -> Option<TrayIcon> {
    // We move the Strings into the closure. This is allowed.
    std::thread::spawn(move || {
        let icon = utils::load_tray_icon();

        // CREATE ITEMS HERE (Inside the thread)
        let quit_item = MenuItem::with_id(MenuId::new(quit_id), "Quit", true, None);
        let settings_item = MenuItem::with_id(MenuId::new(settings_id), "Settings", true, None);
        let capture_item = MenuItem::with_id(MenuId::new(capture_id), "Capture Screen", true, None);

        let tray_menu = Menu::new();
        let _ = tray_menu.append(&capture_item);
        let _ = tray_menu.append(&settings_item);
        let _ = tray_menu.append(&quit_item);

        let _tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("Crab Grab")
            .with_icon(icon)
            .build()
            .unwrap();

        unsafe {
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });
    None
}

/// Linux/macOS: Creates Items on Main Thread.
#[cfg(not(target_os = "windows"))]
fn init_tray_platform(quit_id: String, settings_id: String, capture_id: String) -> Option<TrayIcon> {
    let icon = utils::load_tray_icon();

    // Create items normally
    let quit_item = MenuItem::with_id(MenuId::new(quit_id), "Quit", true, None);
    let settings_item = MenuItem::with_id(MenuId::new(settings_id), "Settings", true, None);
    let capture_item = MenuItem::with_id(MenuId::new(capture_id), "Capture Screen", true, None);

    let tray_menu = Menu::new();
    let _ = tray_menu.append(&capture_item);
    let _ = tray_menu.append(&settings_item);
    let _ = tray_menu.append(&quit_item);

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Crab Grab")
        .with_icon(icon)
        .build()
        .unwrap();

    Some(tray_icon)
}