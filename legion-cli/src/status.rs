use crate::hardware::{battery, custom_mode, fan, lighting, probe, thermal};
use colored::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegionFullStatus {
    pub dmi: probe::DmiInfo,
    pub thermal: Option<thermal::ThermalStatus>,
    pub fans: Option<fan::FanSystemStatus>,
    pub battery: Option<battery::BatteryDetails>,
    pub lighting: Option<lighting::LightingStatus>,
    pub custom: Option<custom_mode::CustomModeConfig>,
}

pub fn get_full_status() -> LegionFullStatus {
    let dmi = probe::probe_dmi();
    let thermal = thermal::get_thermal_status().ok();
    let fans = fan::get_fan_status().ok();
    let battery = battery::get_battery_details().ok();
    let lighting = lighting::get_lighting_status().ok();
    let custom = custom_mode::get_custom_mode_config().ok();

    LegionFullStatus {
        dmi,
        thermal,
        fans,
        battery,
        lighting,
        custom,
    }
}

pub fn print_full_status(status: &LegionFullStatus) {
    println!("{}", "========================================================".cyan().bold());
    println!(
        "{} {}",
        "  Lenovo Legion Hardware Status:".cyan().bold(),
        format!("{} ({})", status.dmi.product_version, status.dmi.product_name).white().bold()
    );
    println!("  BIOS Version: {}", status.dmi.bios_version);
    println!("{}", "========================================================".cyan().bold());
    println!();

    // Thermal profile
    println!("{}", "[Thermal Power Profile]".yellow().bold());
    if let Some(t) = &status.thermal {
        let profile_color = match t.current_profile.as_str() {
            "low-power" | "quiet" => t.current_profile.blue().bold(),
            "balanced" => t.current_profile.white().bold(),
            "performance" => t.current_profile.red().bold(),
            "max-power" => t.current_profile.red().bold(),
            "custom" => t.current_profile.magenta().bold(),
            _ => t.current_profile.white(),
        };
        println!("  Active Profile:    {} (Power LED: {})", profile_color, t.power_led_color);
        println!("  Available:         {}", t.available_choices.join(", "));
    } else {
        println!("  Status:            {}", "Interface unavailable".red());
    }
    println!();

    // Cooling fans
    println!("{}", "[Cooling Fans]".yellow().bold());
    if let Some(f) = &status.fans {
        for fan in &f.fans {
            let mode = if fan.is_manual {
                format!("Manual (Target: {} RPM)", fan.target_rpm).yellow()
            } else {
                "Automatic (Firmware curve)".green()
            };
            println!(
                "  {:<18} {:>4} RPM  [Limits: {}-{} RPM]  Mode: {}",
                fan.label,
                format!("{}", fan.current_rpm).cyan().bold(),
                fan.min_rpm,
                fan.max_rpm,
                mode
            );
        }
    } else {
        println!("  Status:            {}", "Fan telemetry unavailable".red());
    }
    println!();

    // Battery & Power
    println!("{}", "[Battery & Power Subsystem]".yellow().bold());
    if let Some(b) = &status.battery {
        let cm_str = if b.conservation_mode {
            "ENABLED (Capped at 80% to extend lifespan)".green().bold()
        } else {
            "DISABLED (Charging to 100%)".white()
        };
        let health_str = if let Some(h) = b.health_percent {
            format!("{:.1}% ({:.1} Wh / {:.1} Wh)", h, b.energy_full_wh, b.energy_design_wh)
        } else {
            format!("{:.1} Wh", b.energy_full_wh)
        };

        println!("  Capacity:          {}% ({})", format!("{}", b.capacity_percent).green().bold(), b.status);
        println!("  Conservation Mode: {}", cm_str);
        println!("  Battery Health:    {}", health_str);
        println!("  Power Draw:        {:.2} W (Voltage: {:.2} V)", b.power_now_w, b.voltage_now_v);
        if let Some(cycles) = b.cycle_count {
            println!("  Cycle Count:       {}", cycles);
        }
        if let Some(usb) = b.usb_charging {
            println!("  Always-On USB:     {}", if usb { "Enabled".green() } else { "Disabled".white() });
        }
    } else {
        println!("  Status:            {}", "Battery device BAT0 unavailable".red());
    }
    println!();

    // Lighting & Controls
    println!("{}", "[Lighting & Controls]".yellow().bold());
    if let Some(l) = &status.lighting {
        if let Some(lvl) = l.acpi_backlight_level {
            let desc = match lvl {
                0 => "Off",
                1 => "Low",
                2 => "High",
                _ => "Unknown",
            };
            println!("  ACPI Backlight:    Level {} ({})", lvl, desc);
        }
        if let Some(fn_l) = l.fn_lock {
            println!("  Fn-Lock:           {}", if fn_l { "Enabled".green() } else { "Disabled".white() });
        }
        println!(
            "  RGB Controller:    {}",
            if l.rgb_controller_detected {
                "Detected (ITE 8258 USB HID [048d:c195])".green()
            } else {
                "Not detected".yellow()
            }
        );
        if let Some(ref node) = l.hidraw_node {
            println!("  HID Device Node:   {}", node);
        }
        println!(
            "  Write Permissions: {}",
            if l.can_write_hid {
                "Granted (Direct userspace write permitted)".green().bold()
            } else {
                "Restricted (Requires udev rule for unprivileged control)".yellow()
            }
        );
    } else {
        println!("  Status:            {}", "Lighting subsystem unavailable".red());
    }
    println!();

    // Custom Mode / Firmware Tuning
    println!("{}", "[Custom Mode Tuning Attributes]".yellow().bold());
    if let Some(c) = &status.custom {
        if c.supported && !c.attributes.is_empty() {
            for attr in &c.attributes {
                let range = match (attr.min_value, attr.max_value) {
                    (Some(min), Some(max)) => format!(" [Range: {}..{}]", min, max),
                    _ => String::new(),
                };
                let def = attr
                    .default_value
                    .map(|d| format!(" (Default: {})", d))
                    .unwrap_or_default();

                println!(
                    "  {:<24} = {:>4}{}{}",
                    attr.name,
                    format!("{}", attr.current_value).cyan(),
                    range,
                    def.white()
                );
            }
        } else {
            println!("  Status:            {}", "No firmware attributes exposed by platform".yellow());
        }
    } else {
        println!("  Status:            {}", "Custom mode interface unavailable".yellow());
    }

    println!("{}", "========================================================".cyan().bold());
}
