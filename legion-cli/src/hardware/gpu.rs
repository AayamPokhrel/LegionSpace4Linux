use crate::error::{LegionError, Result};
use crate::hardware::custom_mode::{get_custom_mode_config, set_firmware_attribute};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuTelemetry {
    pub available: bool,
    pub name: String,
    pub vbios_version: String,
    pub driver_version: String,
    pub current_temp_c: u32,
    pub target_temp_c: u32,
    pub power_draw_w: f32,
    pub power_limit_w: f32,
    pub min_power_limit_w: f32,
    pub max_power_limit_w: f32,
    pub default_power_limit_w: f32,
    pub graphics_clock_mhz: u32,
    pub max_graphics_clock_mhz: u32,
    pub memory_clock_mhz: u32,
    pub vram_total_mib: u64,
    pub vram_used_mib: u64,
    pub vram_free_mib: u64,
    // Firmware attributes from custom mode
    pub nv_ac_offset_w: Option<i64>,
    pub nv_ctgp_w: Option<i64>,
    pub nv_ppab_w: Option<i64>,
    pub nv_cpu_boost_w: Option<i64>,
}

pub fn get_gpu_telemetry() -> Result<GpuTelemetry> {
    // 1. Query general info via nvidia-smi csv
    let query_cmd = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,vbios_version,driver_version,temperature.gpu,power.draw,power.limit,power.min_limit,power.max_limit,power.default_limit,clocks.current.graphics,clocks.max.graphics,clocks.current.memory,memory.total,memory.used,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output();

    let output = match query_cmd {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
        _ => {
            return Ok(GpuTelemetry {
                available: false,
                name: "NVIDIA Discrete GPU".into(),
                vbios_version: "N/A".into(),
                driver_version: "N/A".into(),
                current_temp_c: 0,
                target_temp_c: 0,
                power_draw_w: 0.0,
                power_limit_w: 0.0,
                min_power_limit_w: 0.0,
                max_power_limit_w: 0.0,
                default_power_limit_w: 0.0,
                graphics_clock_mhz: 0,
                max_graphics_clock_mhz: 0,
                memory_clock_mhz: 0,
                vram_total_mib: 0,
                vram_used_mib: 0,
                vram_free_mib: 0,
                nv_ac_offset_w: None,
                nv_ctgp_w: None,
                nv_ppab_w: None,
                nv_cpu_boost_w: None,
            });
        }
    };

    let fields: Vec<&str> = output.trim().split(',').map(|s| s.trim()).collect();
    if fields.len() < 15 {
        return Err(LegionError::ParseError(
            "Unexpected nvidia-smi telemetry output format".into(),
        ));
    }

    // Query custom mode firmware attributes for NVIDIA hardware
    let fw_cfg = get_custom_mode_config().ok();
    let get_attr = |name: &str| -> Option<i64> {
        fw_cfg
            .as_ref()
            .and_then(|c| c.attributes.iter().find(|a| a.name == name))
            .map(|a| a.current_value)
    };

    let nv_ac_offset_w = get_attr("gpu_nv_ac_offset");
    let nv_ctgp_w = get_attr("gpu_nv_ctgp");
    let nv_ppab_w = get_attr("gpu_nv_ppab");
    let nv_cpu_boost_w = get_attr("gpu_nv_cpu_boost");
    let target_temp_c = get_attr("gpu_temp").map(|v| v as u32).unwrap_or(87);

    Ok(GpuTelemetry {
        available: true,
        name: fields[0].to_string(),
        vbios_version: fields[1].to_string(),
        driver_version: fields[2].to_string(),
        current_temp_c: fields[3].parse().unwrap_or(0),
        target_temp_c,
        power_draw_w: fields[4].parse().unwrap_or(0.0),
        power_limit_w: fields[5].parse().unwrap_or(0.0),
        min_power_limit_w: fields[6].parse().unwrap_or(0.0),
        max_power_limit_w: fields[7].parse().unwrap_or(0.0),
        default_power_limit_w: fields[8].parse().unwrap_or(0.0),
        graphics_clock_mhz: fields[9].parse().unwrap_or(0),
        max_graphics_clock_mhz: fields[10].parse().unwrap_or(0),
        memory_clock_mhz: fields[11].parse().unwrap_or(0),
        vram_total_mib: fields[12].parse().unwrap_or(0),
        vram_used_mib: fields[13].parse().unwrap_or(0),
        vram_free_mib: fields[14].parse().unwrap_or(0),
        nv_ac_offset_w,
        nv_ctgp_w,
        nv_ppab_w,
        nv_cpu_boost_w,
    })
}

pub fn set_gpu_power_limit(watts: f32) -> Result<()> {
    let watts_str = format!("{:.1}", watts);
    let output = Command::new("nvidia-smi")
        .args(["-pl", &watts_str])
        .output()
        .map_err(LegionError::Io)?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(LegionError::InvalidValue(format!(
            "Failed to set GPU power limit: {}",
            err_msg.trim()
        )));
    }

    Ok(())
}

pub fn lock_gpu_clocks(min_mhz: u32, max_mhz: u32) -> Result<()> {
    let clocks_arg = format!("{},{}", min_mhz, max_mhz);
    let output = Command::new("nvidia-smi")
        .args(["-lgc", &clocks_arg])
        .output()
        .map_err(LegionError::Io)?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(LegionError::InvalidValue(format!(
            "Failed to lock GPU clocks: {}",
            err_msg.trim()
        )));
    }

    Ok(())
}

pub fn reset_gpu_clocks() -> Result<()> {
    let output = Command::new("nvidia-smi")
        .args(["-rgc"])
        .output()
        .map_err(LegionError::Io)?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(LegionError::InvalidValue(format!(
            "Failed to reset GPU clocks: {}",
            err_msg.trim()
        )));
    }

    Ok(())
}

pub fn set_gpu_tuning_settings(
    power_limit: Option<f32>,
    gpu_temp: Option<i64>,
    ac_offset: Option<i64>,
    ctgp: Option<i64>,
    ppab: Option<i64>,
    cpu_boost: Option<i64>,
) -> Result<()> {
    if let Some(pl) = power_limit {
        set_gpu_power_limit(pl)?;
    }
    if let Some(t) = gpu_temp {
        set_firmware_attribute("gpu_temp", t)?;
    }
    if let Some(off) = ac_offset {
        set_firmware_attribute("gpu_nv_ac_offset", off)?;
    }
    if let Some(c) = ctgp {
        set_firmware_attribute("gpu_nv_ctgp", c)?;
    }
    if let Some(p) = ppab {
        set_firmware_attribute("gpu_nv_ppab", p)?;
    }
    if let Some(b) = cpu_boost {
        set_firmware_attribute("gpu_nv_cpu_boost", b)?;
    }
    Ok(())
}
