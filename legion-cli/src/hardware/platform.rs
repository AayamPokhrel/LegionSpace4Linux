use crate::error::{LegionError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

const VPC_PATH: &str = "/sys/devices/pci0000:00/0000:00:14.3/PNP0C09:00/VPC2004:00";

fn get_vpc_node(name: &str) -> Option<PathBuf> {
    let p = Path::new(VPC_PATH).join(name);
    if p.exists() {
        return Some(p);
    }
    // Fallback: search /sys/bus/platform/devices/VPC2004:00/
    let alt = Path::new("/sys/bus/platform/devices/VPC2004:00").join(name);
    if alt.exists() {
        return Some(alt);
    }
    None
}

/// Find the USB device directory for the integrated camera (e.g. /sys/bus/usb/devices/5-1)
fn find_camera_usb_path() -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir("/sys/bus/usb/devices") {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Ok(vendor) = fs::read_to_string(p.join("idVendor")) {
                let v = vendor.trim();
                // Known Lenovo camera vendors: 174f (Realtek/Sunplus), 13d3, 04f2 (Chicony), 0408 (Quanta)
                if v == "174f" || v == "13d3" || v == "04f2" || v == "0408" {
                    if p.join("authorized").exists() {
                        return Some(p);
                    }
                }
            }
            if let Ok(prod) = fs::read_to_string(p.join("product")) {
                if prod.to_lowercase().contains("camera") && p.join("authorized").exists() {
                    return Some(p);
                }
            }
        }
    }
    None
}

#[repr(C)]
struct V4L2Control {
    id: u32,
    value: i32,
}

const V4L2_CID_PRIVACY: u32 = 0x009a0910;
// VIDIOC_G_CTRL: _IOWR('V', 27, struct v4l2_control) = 0xc008561b
const VIDIOC_G_CTRL: libc::c_ulong = 0xc008561b;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformFeatures {
    pub camera_power: Option<bool>,
    pub fn_lock: Option<bool>,
    pub usb_charging: Option<bool>,
    pub conservation_mode: Option<bool>,
    pub fan_mode: Option<u32>,
}

pub fn get_platform_features() -> PlatformFeatures {
    let read_bool = |name: &str| -> Option<bool> {
        let node = get_vpc_node(name)?;
        let val = fs::read_to_string(node).ok()?;
        match val.trim() {
            "1" => Some(true),
            "0" => Some(false),
            _ => None,
        }
    };

    let read_u32 = |name: &str| -> Option<u32> {
        let node = get_vpc_node(name)?;
        let val = fs::read_to_string(node).ok()?;
        val.trim().parse::<u32>().ok()
    };

    PlatformFeatures {
        camera_power: get_camera_power().ok(),
        fn_lock: read_bool("fn_lock"),
        usb_charging: read_bool("usb_charging"),
        conservation_mode: read_bool("conservation_mode"),
        fan_mode: read_u32("fan_mode"),
    }
}

pub fn get_camera_power() -> Result<bool> {
    // 1. Check if USB device is unauthorized (software disabled)
    if let Some(usb) = find_camera_usb_path() {
        if let Ok(auth) = fs::read_to_string(usb.join("authorized")) {
            if auth.trim() == "0" {
                return Ok(false);
            }
        }
    }

    // 2. Check V4L2 device node for physical e-shutter privacy state
    for dev in ["/dev/video0", "/dev/video1"] {
        if Path::new(dev).exists() {
            if let Ok(file) = fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(dev)
            {
                let mut ctrl = V4L2Control {
                    id: V4L2_CID_PRIVACY,
                    value: 0,
                };
                unsafe {
                    if libc::ioctl(file.as_raw_fd(), VIDIOC_G_CTRL, &mut ctrl) == 0 {
                        // value == 1 means privacy is ON (shutter closed => camera disabled)
                        // value == 0 means privacy is OFF (shutter open => camera enabled)
                        return Ok(ctrl.value == 0);
                    }
                }
                // If ioctl didn't return privacy ctrl, existence of /dev/video0 means camera is active
                return Ok(true);
            }
        }
    }

    // 3. Fallback to VPC camera_power attribute if present
    if let Some(node) = get_vpc_node("camera_power") {
        if let Ok(val) = fs::read_to_string(&node) {
            if val.trim() == "1" {
                return Ok(true);
            }
        }
    }

    // 4. If /dev/video0 didn't exist but USB was found, shutter might have cut connection
    if find_camera_usb_path().is_some() {
        return Ok(false);
    }

    Ok(false)
}

pub fn set_camera_power(enable: bool) -> Result<bool> {
    let mut modified = false;

    // 1. Set USB authorized state if available
    if let Some(usb) = find_camera_usb_path() {
        let auth_path = usb.join("authorized");
        let val = if enable { "1" } else { "0" };
        if fs::write(&auth_path, val).is_ok() {
            modified = true;
        }
    }

    // 2. Also write to VPC camera_power if available
    if let Some(node) = get_vpc_node("camera_power") {
        let val = if enable { "1" } else { "0" };
        if fs::write(node, val).is_ok() {
            modified = true;
        }
    }

    if !modified {
        return Err(LegionError::UnsupportedFeature(
            "Neither USB camera authorized control nor VPC camera_power was writable".into(),
        ));
    }

    std::thread::sleep(std::time::Duration::from_millis(150));
    get_camera_power()
}
