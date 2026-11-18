use crate::error::{LegionError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

const KBD_BACKLIGHT_PATH: &str =
    "/sys/devices/pci0000:00/0000:00:14.3/PNP0C09:00/VPC2004:00/leds/platform::kbd_backlight/brightness";
const FN_LOCK_PATH: &str =
    "/sys/devices/pci0000:00/0000:00:14.3/PNP0C09:00/VPC2004:00/fn_lock";

#[allow(dead_code)]
const ITE_VID: &str = "048d";
#[allow(dead_code)]
const ITE_PID: &str = "c195";

const REPORT_ID_VENDOR: u8 = 0xCC;
const CMD_COMMIT_PROFILE: u8 = 0x18;

// Lighting effect modes reversed from SELuminousNB64 / SEBasicLighting64
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightingEffect {
    Off,
    Static,
    Breath,
    Wave,
}

impl LightingEffect {
    pub fn to_vendor_code(&self) -> u8 {
        match self {
            LightingEffect::Off => 0x00,
            LightingEffect::Static => 0x01,
            LightingEffect::Breath => 0x03,
            LightingEffect::Wave => 0x09,
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "off" | "none" => Some(LightingEffect::Off),
            "static" | "solid" => Some(LightingEffect::Static),
            "breath" | "breathing" | "pulse" => Some(LightingEffect::Breath),
            "wave" | "rainbow" | "spectrum" => Some(LightingEffect::Wave),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(s: &str) -> Option<Self> {
        let clean = s.trim_start_matches('#').trim();
        match clean.to_lowercase().as_str() {
            "red" => Some(RgbColor::new(255, 0, 0)),
            "green" => Some(RgbColor::new(0, 255, 0)),
            "blue" => Some(RgbColor::new(0, 0, 255)),
            "cyan" => Some(RgbColor::new(0, 211, 254)),
            "magenta" | "purple" => Some(RgbColor::new(255, 0, 255)),
            "yellow" => Some(RgbColor::new(255, 255, 0)),
            "white" => Some(RgbColor::new(255, 255, 255)),
            "orange" => Some(RgbColor::new(255, 128, 0)),
            _ if clean.len() == 6 => {
                let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
                let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
                let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
                Some(RgbColor::new(r, g, b))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightingStatus {
    pub acpi_backlight_level: Option<u32>,
    pub fn_lock: Option<bool>,
    pub rgb_controller_detected: bool,
    pub hidraw_node: Option<String>,
    pub can_write_hid: bool,
}

// Linux ioctl macros
const IOC_NRBITS: u32 = 8;
const IOC_TYPEBITS: u32 = 8;
const IOC_SIZEBITS: u32 = 14;
#[allow(dead_code)]
const IOC_DIRBITS: u32 = 2;

const IOC_NRSHIFT: u32 = 0;
const IOC_TYPESHIFT: u32 = IOC_NRSHIFT + IOC_NRBITS;
const IOC_SIZESHIFT: u32 = IOC_TYPESHIFT + IOC_TYPEBITS;
const IOC_DIRSHIFT: u32 = IOC_SIZESHIFT + IOC_SIZEBITS;

const IOC_READ: u32 = 2;
const IOC_WRITE: u32 = 1;

fn ioc(dir: u32, type_: u8, nr: u8, size: u32) -> libc::c_ulong {
    (((dir as libc::c_ulong) << IOC_DIRSHIFT)
        | ((type_ as libc::c_ulong) << IOC_TYPESHIFT)
        | ((nr as libc::c_ulong) << IOC_NRSHIFT)
        | ((size as libc::c_ulong) << IOC_SIZESHIFT)) as libc::c_ulong
}

fn hidiocsfeature(len: usize) -> libc::c_ulong {
    ioc(IOC_WRITE | IOC_READ, b'H', 0x06, len as u32)
}

pub fn find_ite_hidraw() -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir("/sys/class/hidraw") {
        for entry in entries.flatten() {
            let path = entry.path();
            let dev_link = path.join("device");
            if let Ok(target) = fs::canonicalize(&dev_link) {
                let target_str = target.to_string_lossy();
                // We want interface 0 (the vendor report interface :1.0)
                if (target_str.contains("048D:C195") || target_str.contains("048d:c195"))
                    && target_str.contains(":1.0")
                {
                    let name = entry.file_name();
                    return Some(PathBuf::from("/dev").join(name));
                }
            }
        }
    }
    None
}

pub fn get_lighting_status() -> Result<LightingStatus> {
    let acpi_backlight_level = fs::read_to_string(KBD_BACKLIGHT_PATH)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok());

    let fn_lock = fs::read_to_string(FN_LOCK_PATH)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .map(|v| v == 1);

    let hid_path = find_ite_hidraw();
    let can_write_hid = if let Some(ref p) = hid_path {
        fs::OpenOptions::new().write(true).open(p).is_ok()
    } else {
        false
    };

    Ok(LightingStatus {
        acpi_backlight_level,
        fn_lock,
        rgb_controller_detected: hid_path.is_some(),
        hidraw_node: hid_path.map(|p| p.display().to_string()),
        can_write_hid,
    })
}

pub fn set_acpi_backlight(level: u32) -> Result<()> {
    let p = Path::new(KBD_BACKLIGHT_PATH);
    if !p.exists() {
        return Err(LegionError::UnsupportedFeature("ACPI keyboard backlight interface not found".into()));
    }
    if level > 2 {
        return Err(LegionError::InvalidValue("Brightness level must be 0 (off), 1 (low), or 2 (high)".into()));
    }

    fs::write(p, level.to_string()).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            LegionError::PermissionDenied {
                path: p.display().to_string(),
                action: "write",
            }
        } else {
            LegionError::Io(e)
        }
    })
}

pub fn set_fn_lock(enable: bool) -> Result<()> {
    let p = Path::new(FN_LOCK_PATH);
    if !p.exists() {
        return Err(LegionError::UnsupportedFeature("Fn-Lock interface not found".into()));
    }
    let val = if enable { "1" } else { "0" };

    fs::write(p, val).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            LegionError::PermissionDenied {
                path: p.display().to_string(),
                action: "write",
            }
        } else {
            LegionError::Io(e)
        }
    })
}

pub fn send_hid_feature_report(buf: &[u8]) -> Result<()> {
    let node = find_ite_hidraw().ok_or_else(|| {
        LegionError::DeviceNotFound("ITE 8258 RGB USB HID device not found".into())
    })?;

    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&node)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                LegionError::PermissionDenied {
                    path: node.display().to_string(),
                    action: "write (HID ioctl)",
                }
            } else {
                LegionError::Io(e)
            }
        })?;

    let fd = file.as_raw_fd();
    let req = hidiocsfeature(buf.len());

    let ret = unsafe { libc::ioctl(fd, req, buf.as_ptr()) };
    if ret < 0 {
        return Err(LegionError::Io(std::io::Error::last_os_error()));
    }

    Ok(())
}

/// Send 4-zone RGB packet (32 bytes padded)
pub fn set_zone_color(
    zone: u8,
    effect: LightingEffect,
    speed: u8,
    brightness: u8,
    color: RgbColor,
) -> Result<()> {
    if !(1..=4).contains(&zone) {
        return Err(LegionError::InvalidValue(format!("Invalid zone {}. Must be 1, 2, 3, or 4", zone)));
    }

    let mut buf = [0u8; 32];
    buf[0] = REPORT_ID_VENDOR; // 0xCC
    buf[1] = zone;             // 0x01 - 0x04
    buf[2] = effect.to_vendor_code();
    buf[3] = speed.clamp(1, 4);
    buf[4] = brightness.clamp(1, 3);
    buf[5] = color.r;
    buf[6] = color.g;
    buf[7] = color.b;

    send_hid_feature_report(&buf)
}

/// Set all 4 zones to the specified color and effect
pub fn set_all_zones(
    effect: LightingEffect,
    speed: u8,
    brightness: u8,
    color: RgbColor,
) -> Result<()> {
    for zone in 1..=4 {
        set_zone_color(zone, effect, speed, brightness, color)?;
    }
    // Commit profile
    commit_rgb_profile(1, brightness)?;
    Ok(())
}

/// Send profile commit packet (Subcommand 0x18)
pub fn commit_rgb_profile(profile_idx: u8, brightness: u8) -> Result<()> {
    let mut buf = [0u8; 32];
    buf[0] = REPORT_ID_VENDOR; // 0xCC
    buf[1] = CMD_COMMIT_PROFILE; // 0x18
    buf[2] = profile_idx.clamp(1, 4);
    buf[3] = 1; // flag
    buf[4] = brightness.clamp(1, 3);

    send_hid_feature_report(&buf)
}

pub fn apply_preset(name: &str) -> Result<()> {
    match name.to_lowercase().as_str() {
        "legion-blue" | "cyan" | "default" => {
            let color = RgbColor::new(0, 211, 254);
            set_all_zones(LightingEffect::Static, 2, 3, color)
        }
        "crimson" | "red" => {
            let color = RgbColor::new(255, 0, 0);
            set_all_zones(LightingEffect::Static, 2, 3, color)
        }
        "stealth" | "off" => {
            let color = RgbColor::new(0, 0, 0);
            set_all_zones(LightingEffect::Off, 1, 1, color)
        }
        "cyberpunk" => {
            let cyan = RgbColor::new(0, 211, 254);
            let pink = RgbColor::new(255, 0, 128);
            set_zone_color(1, LightingEffect::Static, 2, 3, cyan)?;
            set_zone_color(2, LightingEffect::Static, 2, 3, cyan)?;
            set_zone_color(3, LightingEffect::Static, 2, 3, pink)?;
            set_zone_color(4, LightingEffect::Static, 2, 3, pink)?;
            commit_rgb_profile(1, 3)
        }
        "spectrum" | "wave" | "rainbow" => {
            let color = RgbColor::new(0, 211, 254);
            set_all_zones(LightingEffect::Wave, 2, 3, color)
        }
        other => Err(LegionError::InvalidValue(format!(
            "Unknown preset '{}'. Available presets: legion-blue, crimson, stealth, cyberpunk, spectrum",
            other
        ))),
    }
}

