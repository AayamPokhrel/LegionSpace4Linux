use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_oled_config_path() -> PathBuf {
    crate::hardware::gpu_modes::get_config_dir().join("oled.json")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OledCareSettings {
    pub master_oled_protection: bool,
    pub pixel_shift_enabled: bool,
    pub taskbar_dimming_enabled: bool,
    pub default_dark_mode: bool,
    pub lcd_overdrive_enabled: bool,
    pub adaptive_refresh_rate: bool,
    pub screen_on_seconds: u64,
    pub last_clean_timestamp: u64,
    pub clean_reminder_due: bool,
}

impl Default for OledCareSettings {
    fn default() -> Self {
        Self {
            master_oled_protection: true,
            pixel_shift_enabled: true,
            taskbar_dimming_enabled: true,
            default_dark_mode: true,
            lcd_overdrive_enabled: false,
            adaptive_refresh_rate: true,
            screen_on_seconds: 7200, // 2 hours initial
            last_clean_timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            clean_reminder_due: false,
        }
    }
}

pub fn load_oled_settings() -> OledCareSettings {
    let path = get_oled_config_path();
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(mut settings) = serde_json::from_str::<OledCareSettings>(&data) {
            // Check if clean reminder is due (>= 4 hours = 14400 seconds)
            settings.clean_reminder_due = settings.screen_on_seconds >= 14400;
            return settings;
        }
    }
    OledCareSettings::default()
}

pub fn save_oled_settings(settings: &OledCareSettings) {
    let path = get_oled_config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
        crate::hardware::gpu_modes::fix_file_owner(parent);
    }
    if let Ok(data) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(&path, data);
        crate::hardware::gpu_modes::fix_file_owner(&path);
    }
}

pub fn set_system_dark_mode(enable: bool) -> Result<()> {
    let mode = if enable { "'prefer-dark'" } else { "'default'" };
    let _ = Command::new("gsettings")
        .args(["set", "org.gnome.desktop.interface", "color-scheme", mode])
        .status();
    Ok(())
}

pub fn reset_pixel_clean_timer() -> Result<OledCareSettings> {
    let mut settings = load_oled_settings();
    settings.screen_on_seconds = 0;
    settings.clean_reminder_due = false;
    settings.last_clean_timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
    save_oled_settings(&settings);
    Ok(settings)
}

pub fn update_oled_settings(
    master: Option<bool>,
    pixel_shift: Option<bool>,
    taskbar_dim: Option<bool>,
    dark_mode: Option<bool>,
    overdrive: Option<bool>,
    adaptive_rr: Option<bool>,
) -> Result<OledCareSettings> {
    let mut settings = load_oled_settings();

    if let Some(m) = master {
        settings.master_oled_protection = m;
    }
    if let Some(ps) = pixel_shift {
        settings.pixel_shift_enabled = ps;
    }
    if let Some(td) = taskbar_dim {
        settings.taskbar_dimming_enabled = td;
    }
    if let Some(dm) = dark_mode {
        settings.default_dark_mode = dm;
        let _ = set_system_dark_mode(dm);
    }
    if let Some(od) = overdrive {
        settings.lcd_overdrive_enabled = od;
    }
    if let Some(arr) = adaptive_rr {
        settings.adaptive_refresh_rate = arr;
    }

    save_oled_settings(&settings);
    Ok(settings)
}
