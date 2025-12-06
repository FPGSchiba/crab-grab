use std::str::FromStr;
use serde::{Deserialize, Serialize};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};

// TASK: Add #[derive(Serialize, Deserialize)] macros
// Note: 'HotKey' might not implement Serialize/Deserialize by default!
// If it doesn't, we have a problem.
// WORKAROUND: We shouldn't save the 'HotKey' struct directly.
// Instead, we save the 'text representation' (e.g. "Ctrl+Shift+G") or the raw KeyCode enum.
// For now, let's mark 'snap_hotkey' to be skipped by Serde and reconstructed manually,
// OR create a 'SavedConfig' struct that mirrors AppConfig but uses strings for keys.

// 1. Helper function for the "default" attribute
fn default_snap_key() -> HotKey {
    HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyG)
}

fn hotkey_to_savable(hotkey: &HotKey) -> (String, u32) {
    (hotkey.key.to_string(), hotkey.mods.bits())
}

fn savable_to_hotkey(code: &str, modifiers: u32) -> HotKey {
    let mods = Modifiers::from_bits(modifiers);
    if let Ok(key) = Code::from_str(code) {
        HotKey::new(mods, key)
    } else {
        // Fallback to default if parsing fails
        default_snap_key()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub save_directory: String,
    pub auto_save: bool,
    pub play_sound: bool,
    pub custom_cursor: bool,
    pub run_on_startup: bool,

    // 2. The Runtime Hotkey (Skipped by Serde)
    // We tell Serde: "If this is missing, call default_snap_key() to make one"
    #[serde(skip, default = "default_snap_key")]
    pub snap_hotkey: HotKey,

    // 3. The Saved Data (u32 is easy to save/load)
    // We will sync these with the 'snap_hotkey' before saving/after loading
    pub snap_hotkey_mods: u32,
    pub snap_hotkey_code: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            save_directory: dirs::picture_dir().unwrap().to_string_lossy().to_string(),
            auto_save: false,
            play_sound: true,
            custom_cursor: true,
            run_on_startup: false,
            snap_hotkey: default_snap_key(),
            // Sync the raw numbers with the default key
            snap_hotkey_mods: (Modifiers::CONTROL | Modifiers::SHIFT).bits(),
            snap_hotkey_code: Code::KeyG.to_string(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("crab_config.json");
            return if let Ok(data) = std::fs::read_to_string(config_path) {
                if let Ok(mut config) = serde_json::from_str::<AppConfig>(&data) {
                    let snap_hotkey = savable_to_hotkey(&config.snap_hotkey_code, config.snap_hotkey_mods);
                    config.snap_hotkey = snap_hotkey;
                    config
                } else {
                    eprintln!("Failed to parse config file, using default config.");
                    AppConfig::default()
                }
            } else {
                eprintln!("Config file not found, using default config.");
                AppConfig::default()
            }
        } else {
            eprintln!("Could not determine config directory, using default config.");
        }
        AppConfig::default()
    }

    pub fn save(&mut self) {
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("crab_config.json");
            let (code_str, mods_bits) = hotkey_to_savable(&self.snap_hotkey);
            self.snap_hotkey_code = code_str;
            self.snap_hotkey_mods = mods_bits;
            if let Ok(json) = serde_json::to_string_pretty(&self) {
                if let Err(e) = std::fs::create_dir_all(&config_dir) {
                    eprintln!("Failed to create config directory: {}", e);
                    return;
                }
                if let Err(e) = std::fs::write(config_path, json) {
                    eprintln!("Failed to write config file: {}", e);
                }
            } else {
                eprintln!("Failed to serialize config.");
            }
        } else {
            eprintln!("Could not determine config directory, config not saved.");
        }
    }
}

