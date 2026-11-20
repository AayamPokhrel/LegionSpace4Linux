use crate::cli::*;
use crate::error::{LegionError, Result};
use crate::hardware::{battery, custom_mode, fan, gpu, gpu_modes, lighting, oled, platform, probe, thermal};
use crate::status;
use colored::*;
use serde::Serialize;
use std::process;

#[derive(Serialize)]
struct ActionResponse<T: Serialize> {
    success: bool,
    action: &'static str,
    data: T,
}

#[derive(Serialize)]
struct ErrorResponse {
    success: bool,
    error: String,
}

pub fn run(cli: Cli) {
    if let Err(err) = dispatch(&cli) {
        if cli.json {
            let resp = ErrorResponse {
                success: false,
                error: err.to_string(),
            };
            if let Ok(s) = serde_json::to_string_pretty(&resp) {
                eprintln!("{}", s);
            } else {
                eprintln!("{{\"success\":false,\"error\":\"{}\"}}", err);
            }
        } else {
            eprintln!();
            eprintln!("{} {}", "Error:".red().bold(), err);
            if let LegionError::PermissionDenied { path, action: _ } = &err {
                eprintln!();
                eprintln!("{}", "--- Permission Denied Guidance ---".yellow().bold());
                eprintln!(
                    "Writing to '{}' requires elevated privileges or proper udev rules.",
                    path
                );
                eprintln!("To grant standard user access without sudo, install the Legion udev rules:");
                eprintln!("  sudo cp udev/99-lenovo-legion.rules /etc/udev/rules.d/");
                eprintln!("  sudo udevadm control --reload && sudo udevadm trigger");
                eprintln!();
                eprintln!("On NixOS, enable the native Legion module in configuration.nix:");
                eprintln!("  hardware.lenovo-legion.enable = true;");
                eprintln!("----------------------------------");
            }
        }
        process::exit(1);
    }
}

fn dispatch(cli: &Cli) -> Result<()> {
    match &cli.command {
        None | Some(Commands::Status) => handle_status(cli.json),
        Some(Commands::Probe) => handle_probe(cli.json),
        Some(Commands::Profile(cmd)) => handle_profile(cmd, cli.json),
        Some(Commands::Fan(cmd)) => handle_fan(cmd, cli.json),
        Some(Commands::Battery(cmd)) => handle_battery(cmd, cli.json),
        Some(Commands::Custom(cmd)) => handle_custom(cmd, cli.json),
        Some(Commands::Gpu(cmd)) => handle_gpu(cmd, cli.json),
        Some(Commands::Camera(cmd)) => handle_camera(cmd, cli.json),
        Some(Commands::Oled(cmd)) => handle_oled(cmd, cli.json),
        Some(Commands::FnLock(cmd)) => handle_fn_lock(cmd, cli.json),
        Some(Commands::Lighting(cmd)) => handle_lighting(cmd, cli.json),
    }
}

// ----------------------------------------------------------------------------
// Status & Probe Handlers
// ----------------------------------------------------------------------------
fn handle_status(json: bool) -> Result<()> {
    let s = status::get_full_status();
    if json {
        println!("{}", serde_json::to_string_pretty(&s).unwrap());
    } else {
        status::print_full_status(&s);
    }
    Ok(())
}

fn handle_probe(json: bool) -> Result<()> {
    let report = probe::run_full_probe();
    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        probe::print_probe_report(&report);
    }
    Ok(())
}

// ----------------------------------------------------------------------------
// Thermal Profile Handlers
// ----------------------------------------------------------------------------
fn handle_profile(cmd: &ProfileCmd, json: bool) -> Result<()> {
    match &cmd.action {
        None | Some(ProfileAction::Get) => {
            let status = thermal::get_thermal_status()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&status).unwrap());
            } else {
                println!(
                    "Active Profile: {} (Power LED: {})",
                    status.current_profile.cyan().bold(),
                    status.power_led_color.white().bold()
                );
            }
        }
        Some(ProfileAction::Set { name }) => {
            let updated = thermal::set_thermal_profile(name)?;
            if json {
                let resp = ActionResponse {
                    success: true,
                    action: "profile_set",
                    data: &updated,
                };
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else {
                println!(
                    "{} Switched thermal profile to '{}' (Power LED: {})",
                    "[OK]".green().bold(),
                    updated.current_profile.cyan().bold(),
                    updated.power_led_color.white().bold()
                );
            }
        }
        Some(ProfileAction::List) => {
            let status = thermal::get_thermal_status()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&status.available_choices).unwrap());
            } else {
                println!("{}", "Available Thermal Profiles:".cyan().bold());
                for choice in &status.available_choices {
                    let led = thermal::map_led_color(choice);
                    if choice == &status.current_profile {
                        println!(
                            "  * {:<14} (Power LED: {:<11}) {}",
                            choice.green().bold(),
                            led,
                            "[ACTIVE]".cyan().bold()
                        );
                    } else {
                        println!("  - {:<14} (Power LED: {})", choice, led);
                    }
                }
            }
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------------
// Fan Handlers
// ----------------------------------------------------------------------------
fn handle_fan(cmd: &FanCmd, json: bool) -> Result<()> {
    match &cmd.action {
        None | Some(FanAction::Status) => {
            let status = fan::get_fan_status()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&status).unwrap());
            } else {
                println!("{}", "Cooling Fan Telemetry:".cyan().bold());
                for f in &status.fans {
                    let mode_str = if f.is_manual {
                        format!("Manual Target: {} RPM", f.target_rpm).yellow()
                    } else {
                        "Automatic Curve".green()
                    };
                    println!(
                        "  {:<16} Current: {:>4} RPM  [Limits: {}-{} RPM]  Mode: {}",
                        f.label,
                        format!("{}", f.current_rpm).cyan().bold(),
                        f.min_rpm,
                        f.max_rpm,
                        mode_str
                    );
                }
            }
        }
        Some(FanAction::Set { fan1, fan2, both }) => {
            if let Some(target) = both {
                fan::set_both_fans(*target, *target)?;
            } else {
                if let Some(target1) = fan1 {
                    fan::set_fan_target(1, *target1)?;
                }
                if let Some(target2) = fan2 {
                    fan::set_fan_target(2, *target2)?;
                }
            }
            let status = fan::get_fan_status()?;
            if json {
                let resp = ActionResponse {
                    success: true,
                    action: "fan_set",
                    data: &status,
                };
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else {
                println!("{} Fan speed targets updated successfully.", "[OK]".green().bold());
                for f in &status.fans {
                    println!("  {}: target set to {} RPM", f.label, f.target_rpm);
                }
            }
        }
        Some(FanAction::Auto) => {
            let status = fan::set_auto()?;
            if json {
                let resp = ActionResponse {
                    success: true,
                    action: "fan_auto",
                    data: &status,
                };
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else {
                println!(
                    "{} Restored automatic firmware fan control curves.",
                    "[OK]".green().bold()
                );
            }
        }
        Some(FanAction::Full) => {
            let status = fan::set_full_speed()?;
            if json {
                let resp = ActionResponse {
                    success: true,
                    action: "fan_full_speed",
                    data: &status,
                };
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else {
                println!(
                    "{} Fans set to maximum rated speed (5100 RPM).",
                    "[OK]".green().bold()
                );
            }
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------------
// Battery Handlers
// ----------------------------------------------------------------------------
fn handle_battery(cmd: &BatteryCmd, json: bool) -> Result<()> {
    match &cmd.action {
        None | Some(BatteryAction::Status) => {
            let details = battery::get_battery_details()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&details).unwrap());
            } else {
                println!("{}", "Battery & Power Subsystem:".cyan().bold());
                println!("  Capacity:          {}% ({})", details.capacity_percent, details.status);
                let cm_str = if details.conservation_mode {
                    "ENABLED (Capped at 80% to protect battery lifespan)".green().bold()
                } else {
                    "DISABLED (Normal charging to 100%)".white()
                };
                println!("  Conservation Mode: {}", cm_str);
                if let Some(h) = details.health_percent {
                    println!(
                        "  Battery Health:    {:.1}% ({:.1} Wh / {:.1} Wh)",
                        h, details.energy_full_wh, details.energy_design_wh
                    );
                }
                println!("  Power Rate:        {:.2} W", details.power_now_w);
                println!("  Voltage:           {:.2} V", details.voltage_now_v);
                if let Some(c) = details.cycle_count {
                    println!("  Cycle Count:       {}", c);
                }
                if let Some(usb) = details.usb_charging {
                    println!("  Always-On USB:     {}", if usb { "Enabled" } else { "Disabled" });
                }
            }
        }
        Some(BatteryAction::Conservation { state }) => {
            match state {
                Some(st) => {
                    let enable = st.is_enabled();
                    let updated = battery::set_conservation_mode(enable)?;
                    if json {
                        let resp = ActionResponse {
                            success: true,
                            action: "battery_conservation_set",
                            data: &updated,
                        };
                        println!("{}", serde_json::to_string_pretty(&resp).unwrap());
                    } else {
                        let msg = if enable {
                            "Enabled (Charging capped at ~80% for battery preservation)"
                        } else {
                            "Disabled (Full 100% charging enabled)"
                        };
                        println!("{} Battery Conservation Mode: {}", "[OK]".green().bold(), msg.cyan().bold());
                    }
                }
                None => {
                    let details = battery::get_battery_details()?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "conservation_mode": details.conservation_mode
                            }))
                            .unwrap()
                        );
                    } else {
                        println!(
                            "Conservation Mode is currently: {}",
                            if details.conservation_mode {
                                "ENABLED".green().bold()
                            } else {
                                "DISABLED".white()
                            }
                        );
                    }
                }
            }
        }
        Some(BatteryAction::UsbCharge { state }) => {
            match state {
                Some(st) => {
                    let enable = st.is_enabled();
                    let updated = battery::set_usb_charging(enable)?;
                    if json {
                        let resp = ActionResponse {
                            success: true,
                            action: "battery_usb_charging_set",
                            data: &updated,
                        };
                        println!("{}", serde_json::to_string_pretty(&resp).unwrap());
                    } else {
                        println!(
                            "{} Always-on USB charging set to: {}",
                            "[OK]".green().bold(),
                            if enable { "Enabled".green() } else { "Disabled".white() }
                        );
                    }
                }
                None => {
                    let details = battery::get_battery_details()?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "usb_charging": details.usb_charging
                            }))
                            .unwrap()
                        );
                    } else {
                        println!(
                            "Always-on USB charging is currently: {:?}",
                            details.usb_charging
                        );
                    }
                }
            }
        }
        Some(BatteryAction::RapidCharge { state }) => {
            match state {
                Some(st) => {
                    let enable = st.is_enabled();
                    platform::set_rapid_charge(enable)?;
                    if json {
                        let resp = ActionResponse {
                            success: true,
                            action: "battery_rapid_charge_set",
                            data: enable,
                        };
                        println!("{}", serde_json::to_string_pretty(&resp).unwrap());
                    } else {
                        println!(
                            "{} Rapid Charge mode set to: {}",
                            "[OK]".green().bold(),
                            if enable { "Enabled (0-70% in 30 mins)".green() } else { "Disabled".white() }
                        );
                    }
                }
                None => {
                    let rapid = platform::get_rapid_charge();
                    if json {
                        println!("{}", serde_json::json!({ "rapid_charge": rapid }));
                    } else {
                        println!(
                            "Rapid Charge is currently: {}",
                            if rapid { "ENABLED".green().bold() } else { "DISABLED".white() }
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------------
// Custom Mode Handlers
// ----------------------------------------------------------------------------
fn handle_custom(cmd: &CustomCmd, json: bool) -> Result<()> {
    match &cmd.action {
        None | Some(CustomAction::Status) => {
            let conf = custom_mode::get_custom_mode_config()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&conf).unwrap());
            } else {
                println!("{}", "Custom Mode Firmware Attributes:".cyan().bold());
                for attr in &conf.attributes {
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
                        format!("{}", attr.current_value).cyan().bold(),
                        range,
                        def.white()
                    );
                }
            }
        }
        Some(CustomAction::Set {
            name,
            value,
            pl1,
            pl2,
            pl3,
            cpu_cl,
            cpu_temp,
            gpu_temp,
            nv_offset,
            ctgp,
            ppab,
            cpu_boost,
            tgp_offset,
        }) => {
            let mut changes = Vec::new();

            if let (Some(n), Some(v)) = (name, value) {
                custom_mode::set_firmware_attribute(n, *v)?;
                changes.push(format!("{} = {}", n, v));
            }
            if let Some(v) = pl1 {
                custom_mode::set_firmware_attribute("ppt_pl1_spl", *v)?;
                changes.push(format!("ppt_pl1_spl = {} W", v));
            }
            if let Some(v) = pl2 {
                custom_mode::set_firmware_attribute("ppt_pl2_sppt", *v)?;
                changes.push(format!("ppt_pl2_sppt = {} W", v));
            }
            if let Some(v) = pl3 {
                custom_mode::set_firmware_attribute("ppt_pl3_fppt", *v)?;
                changes.push(format!("ppt_pl3_fppt = {} W", v));
            }
            if let Some(v) = cpu_cl {
                custom_mode::set_firmware_attribute("ppt_cpu_cl", *v)?;
                changes.push(format!("ppt_cpu_cl = {} W", v));
            }
            if let Some(v) = cpu_temp {
                custom_mode::set_firmware_attribute("cpu_temp", *v)?;
                changes.push(format!("cpu_temp = {} °C", v));
            }
            if let Some(v) = gpu_temp {
                custom_mode::set_firmware_attribute("gpu_temp", *v)?;
                changes.push(format!("gpu_temp = {} °C", v));
            }
            if let Some(v) = nv_offset.or(*tgp_offset) {
                custom_mode::set_firmware_attribute("gpu_nv_ac_offset", v)?;
                changes.push(format!("gpu_nv_ac_offset = {} W", v));
            }
            if let Some(v) = ctgp {
                custom_mode::set_firmware_attribute("gpu_nv_ctgp", *v)?;
                changes.push(format!("gpu_nv_ctgp = {} W", v));
            }
            if let Some(v) = ppab {
                custom_mode::set_firmware_attribute("gpu_nv_ppab", *v)?;
                changes.push(format!("gpu_nv_ppab = {} W", v));
            }
            if let Some(v) = cpu_boost {
                custom_mode::set_firmware_attribute("gpu_nv_cpu_boost", *v)?;
                changes.push(format!("gpu_nv_cpu_boost = {} W", v));
            }

            if changes.is_empty() {
                return Err(LegionError::InvalidValue(
                    "No attribute options provided. Use --name and --value, or --pl1, --pl2, --ctgp, --ppab, etc."
                        .into(),
                ));
            }

            let conf = custom_mode::get_custom_mode_config()?;
            if json {
                let resp = ActionResponse {
                    success: true,
                    action: "custom_attributes_set",
                    data: &conf,
                };
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else {
                println!("{} Firmware attributes updated:", "[OK]".green().bold());
                for change in changes {
                    println!("  {}", change.cyan());
                }
            }
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------------
// GPU Handlers
// ----------------------------------------------------------------------------
fn handle_gpu(cmd: &GpuCmd, json: bool) -> Result<()> {
    match &cmd.action {
        None | Some(GpuAction::Status) => {
            let telem = gpu::get_gpu_telemetry()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&telem).unwrap());
            } else {
                println!("{}", "NVIDIA Discrete GPU Status:".cyan().bold());
                if !telem.available {
                    println!("  GPU is currently unavailable or powered down.");
                } else {
                    println!("  Model:              {}", telem.name.green().bold());
                    println!("  VBIOS Version:      {}", telem.vbios_version);
                    println!("  Driver Version:     {}", telem.driver_version);
                    println!(
                        "  Temperature:        {} °C (Ceiling: {} °C)",
                        telem.current_temp_c, telem.target_temp_c
                    );
                    println!(
                        "  Power Draw:         {:.2} W (TGP Limit: {:.1} W, Max: {:.1} W)",
                        telem.power_draw_w, telem.power_limit_w, telem.max_power_limit_w
                    );
                    println!(
                        "  Graphics Clock:     {} MHz (Max: {} MHz)",
                        telem.graphics_clock_mhz, telem.max_graphics_clock_mhz
                    );
                    println!("  Memory Clock:       {} MHz", telem.memory_clock_mhz);
                    println!(
                        "  VRAM Memory:        {} MiB / {} MiB ({:.1}% used)",
                        telem.vram_used_mib,
                        telem.vram_total_mib,
                        (telem.vram_used_mib as f64 / telem.vram_total_mib.max(1) as f64) * 100.0
                    );
                    if let Some(c) = telem.nv_ctgp_w {
                        println!("  Configurable TGP:   {} W", c);
                    }
                    if let Some(p) = telem.nv_ppab_w {
                        println!("  Dynamic Boost:      {} W", p);
                    }
                    if let Some(o) = telem.nv_ac_offset_w {
                        println!("  AC Baseline Offset: {} W", o);
                    }
                }
            }
        }
        Some(GpuAction::Set {
            power_limit,
            gpu_temp,
            ac_offset,
            ctgp,
            ppab,
            cpu_boost,
        }) => {
            gpu::set_gpu_tuning_settings(*power_limit, *gpu_temp, *ac_offset, *ctgp, *ppab, *cpu_boost)?;
            let telem = gpu::get_gpu_telemetry()?;
            if json {
                let resp = ActionResponse {
                    success: true,
                    action: "gpu_tuning_set",
                    data: &telem,
                };
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else {
                println!("{} Updated GPU tuning parameters successfully.", "[OK]".green().bold());
            }
        }
        Some(GpuAction::LockClocks { min, max }) => {
            gpu::lock_gpu_clocks(*min, *max)?;
            if json {
                println!("{}", serde_json::json!({"success": true, "locked_clocks": [min, max]}));
            } else {
                println!("{} Locked GPU clocks to {}-{} MHz.", "[OK]".green().bold(), min, max);
            }
        }
        Some(GpuAction::ResetClocks) => {
            gpu::reset_gpu_clocks()?;
            if json {
                println!("{}", serde_json::json!({"success": true, "reset_clocks": true}));
            } else {
                println!("{} Reset GPU clocks to automatic management.", "[OK]".green().bold());
            }
        }
        Some(GpuAction::Mode { mode }) => {
            match mode {
                Some(m_str) => {
                    let target_mode: gpu_modes::GpuWorkingMode = m_str.parse()?;
                    let res = gpu_modes::set_gpu_working_mode(target_mode)?;
                    if json {
                        let resp = ActionResponse {
                            success: true,
                            action: "gpu_mode_set",
                            data: &res,
                        };
                        println!("{}", serde_json::to_string_pretty(&resp).unwrap());
                    } else {
                        println!("{} GPU Working Mode set to: {}", "[OK]".green().bold(), res.active_mode.to_string().cyan().bold());
                        println!("  MUX Switch:        {}", res.mux_status);
                        println!("  Runtime PM Status: {}", res.dgpu_power_state);
                    }
                }
                None => {
                    let st = gpu_modes::get_gpu_mode_status();
                    if json {
                        println!("{}", serde_json::to_string_pretty(&st).unwrap());
                    } else {
                        println!("{}", "GPU Working Mode & MUX Status:".cyan().bold());
                        println!("  Active Mode:       {}", st.active_mode.to_string().green().bold());
                        println!("  Hardware MUX:      {}", st.mux_status);
                        println!("  dGPU Power State:  {}", st.dgpu_power_state);
                        println!("  Power Source:      {}", if st.ac_connected { "AC Adapter" } else { "Battery (DC)" });
                        if let Some(pm) = st.powermizer_mode {
                            let pm_desc = match pm {
                                1 => "Prefer Maximum Performance (1)",
                                2 => "Auto (2)",
                                _ => "Adaptive (0)",
                            };
                            println!("  PowerMizer Mode:   {}", pm_desc);
                        }
                    }
                }
            }
        }
        Some(GpuAction::Powermizer { level }) => {
            match level {
                Some(lvl) => {
                    gpu_modes::set_powermizer_mode(*lvl)?;
                    if json {
                        println!("{}", serde_json::json!({"success": true, "powermizer_mode": lvl}));
                    } else {
                        println!("{} Set GPUPowerMizerMode to {}.", "[OK]".green().bold(), lvl);
                    }
                }
                None => {
                    let mode = gpu_modes::get_powermizer_mode().unwrap_or(0);
                    if json {
                        println!("{}", serde_json::json!({"powermizer_mode": mode}));
                    } else {
                        println!("GPUPowerMizerMode is currently: {}", mode);
                    }
                }
            }
        }
        Some(GpuAction::Apps) => {
            let procs = gpu_modes::get_gpu_processes();
            if json {
                println!("{}", serde_json::to_string_pretty(&procs).unwrap());
            } else {
                println!("{}", "Applications Utilizing Discrete NVIDIA GPU:".cyan().bold());
                if procs.is_empty() {
                    println!("  No active compute/graphics applications detected on dGPU.");
                } else {
                    for p in procs {
                        println!("  PID {:<7} {:<30} (VRAM: {} MiB)", p.pid, p.name.green(), p.used_memory_mib);
                    }
                }
            }
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------------
// OLED & Display Care Handlers
// ----------------------------------------------------------------------------
fn handle_oled(cmd: &OledCmd, json: bool) -> Result<()> {
    match &cmd.action {
        None | Some(OledAction::Status) => {
            let settings = oled::load_oled_settings();
            if json {
                println!("{}", serde_json::to_string_pretty(&settings).unwrap());
            } else {
                println!("{}", "OLED & Display Care Protection Suite:".cyan().bold());
                println!("  Master OLED Protection:  {}", if settings.master_oled_protection { "Enabled".green() } else { "Disabled".yellow() });
                println!("  Pixel Shift (Anti-Burn): {}", if settings.pixel_shift_enabled { "Enabled".green() } else { "Disabled".yellow() });
                println!("  Static Taskbar Dimming:  {}", if settings.taskbar_dimming_enabled { "Enabled".green() } else { "Disabled".yellow() });
                println!("  Default Dark Mode:       {}", if settings.default_dark_mode { "Enabled".green() } else { "Disabled".yellow() });
                println!("  LCD Panel OverDrive:     {}", if settings.lcd_overdrive_enabled { "Enabled".green() } else { "Disabled".yellow() });
                println!("  Adaptive Refresh Rate:   {}", if settings.adaptive_refresh_rate { "Enabled".green() } else { "Disabled".yellow() });
                let hours = settings.screen_on_seconds as f64 / 3600.0;
                println!("  Panel On-Time (Tracker): {:.1} hours / 4.0 hours", hours);
                println!("  Pixel Clean Cycle Due:   {}", if settings.clean_reminder_due { "YES (Run Clean Cycle!)".red().bold() } else { "No".green() });
            }
        }
        Some(OledAction::Set {
            master,
            pixel_shift,
            taskbar_dim,
            dark_mode,
            overdrive,
            adaptive_rr,
        }) => {
            let updated = oled::update_oled_settings(
                master.map(|s| s.is_enabled()),
                pixel_shift.map(|s| s.is_enabled()),
                taskbar_dim.map(|s| s.is_enabled()),
                dark_mode.map(|s| s.is_enabled()),
                overdrive.map(|s| s.is_enabled()),
                adaptive_rr.map(|s| s.is_enabled()),
            )?;
            if json {
                let resp = ActionResponse {
                    success: true,
                    action: "oled_settings_updated",
                    data: &updated,
                };
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else {
                println!("{} OLED Care settings successfully updated.", "[OK]".green().bold());
            }
        }
        Some(OledAction::Clean) => {
            let updated = oled::reset_pixel_clean_timer()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&updated).unwrap());
            } else {
                println!("{} OLED panel pixel clean maintenance cycle completed & timer reset.", "[OK]".green().bold());
            }
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------------
// Fn-Lock Handlers
// ----------------------------------------------------------------------------
fn handle_fn_lock(cmd: &FnLockCmd, json: bool) -> Result<()> {
    match cmd.state {
        Some(st) => {
            let enable = st.is_enabled();
            platform::set_fn_lock(enable)?;
            if json {
                println!("{}", serde_json::json!({"success": true, "fn_lock": enable}));
            } else {
                println!("{} Fn-Lock set to: {}", "[OK]".green().bold(), if enable { "Enabled (F1-F12 keys direct)" } else { "Disabled (Media hotkeys direct)" });
            }
        }
        None => {
            let state = platform::get_platform_features().fn_lock.unwrap_or(false);
            if json {
                println!("{}", serde_json::json!({"fn_lock": state}));
            } else {
                println!("Fn-Lock is currently: {}", if state { "Enabled (F1-F12 direct)".green() } else { "Disabled (Hotkeys direct)".white() });
            }
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------------
// Camera Handlers
// ----------------------------------------------------------------------------
fn handle_camera(cmd: &CameraCmd, json: bool) -> Result<()> {
    match cmd.state {
        Some(st) => {
            let enable = st.is_enabled();
            let updated = platform::set_camera_power(enable)?;
            if json {
                let resp = ActionResponse {
                    success: true,
                    action: "camera_power_set",
                    data: updated,
                };
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else {
                println!(
                    "{} Camera e-shutter power: {}",
                    "[OK]".green().bold(),
                    if updated { "Enabled (shutter open)".green() } else { "Disabled (shutter killed)".yellow() }
                );
            }
        }
        None => {
            let state = platform::get_camera_power()?;
            if json {
                println!("{}", serde_json::json!({"camera_power": state}));
            } else {
                println!(
                    "Camera hardware e-shutter is currently: {}",
                    if state { "Enabled (shutter open)".green().bold() } else { "Disabled (shutter killed)".yellow().bold() }
                );
            }
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------------
// Lighting Handlers
// ----------------------------------------------------------------------------
fn handle_lighting(cmd: &LightingCmd, json: bool) -> Result<()> {
    match &cmd.action {
        None | Some(LightingAction::Status) => {
            let status = lighting::get_lighting_status()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&status).unwrap());
            } else {
                println!("{}", "Lighting & Input Subsystem:".cyan().bold());
                if let Some(lvl) = status.acpi_backlight_level {
                    println!("  ACPI Backlight:    Level {} (0=Off, 1=Low, 2=High)", lvl);
                }
                if let Some(fn_l) = status.fn_lock {
                    println!("  Fn-Lock:           {}", if fn_l { "Enabled" } else { "Disabled" });
                }
                println!(
                    "  RGB Controller:    {}",
                    if status.rgb_controller_detected {
                        "Detected (ITE 8258 USB HID)".green()
                    } else {
                        "Not detected".yellow()
                    }
                );
                if let Some(ref node) = status.hidraw_node {
                    println!("  HID Node:          {}", node);
                }
                println!(
                    "  Write Permissions: {}",
                    if status.can_write_hid {
                        "Granted (Ready for RGB control)".green().bold()
                    } else {
                        "Restricted (Install udev rules for unprivileged access)".yellow()
                    }
                );
            }
        }
        Some(LightingAction::KbdBacklight { level }) => {
            lighting::set_acpi_backlight(*level)?;
            let status = lighting::get_lighting_status()?;
            if json {
                let resp = ActionResponse {
                    success: true,
                    action: "kbd_backlight_set",
                    data: &status,
                };
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else {
                println!("{} Keyboard backlight set to level {}", "[OK]".green().bold(), level);
            }
        }
        Some(LightingAction::FnLock { state }) => {
            lighting::set_fn_lock(state.is_enabled())?;
            let status = lighting::get_lighting_status()?;
            if json {
                let resp = ActionResponse {
                    success: true,
                    action: "fn_lock_set",
                    data: &status,
                };
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else {
                println!(
                    "{} Fn-Lock set to {}",
                    "[OK]".green().bold(),
                    if state.is_enabled() { "Enabled" } else { "Disabled" }
                );
            }
        }
        Some(LightingAction::Static {
            zone,
            color,
            brightness,
        }) => {
            let rgb = lighting::RgbColor::from_hex(color).ok_or_else(|| {
                LegionError::InvalidValue(format!("Invalid color code or name: '{}'", color))
            })?;

            if zone.to_lowercase() == "all" {
                lighting::set_all_zones(lighting::LightingEffect::Static, 2, *brightness, rgb)?;
            } else {
                let z: u8 = zone.parse().map_err(|_| {
                    LegionError::InvalidValue(format!("Invalid zone '{}'. Expected 1-4 or 'all'", zone))
                })?;
                lighting::set_zone_color(z, lighting::LightingEffect::Static, 2, *brightness, rgb)?;
                lighting::commit_rgb_profile(1, *brightness)?;
            }

            if json {
                let resp = serde_json::json!({
                    "success": true,
                    "action": "lighting_static",
                    "zone": zone,
                    "color": color,
                    "brightness": brightness
                });
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else {
                println!(
                    "{} Set static color {} on zone {} (brightness: {})",
                    "[OK]".green().bold(),
                    color.cyan(),
                    zone,
                    brightness
                );
            }
        }
        Some(LightingAction::Effect {
            mode,
            zone,
            color,
            speed,
            brightness,
        }) => {
            let effect = lighting::LightingEffect::from_str(mode).ok_or_else(|| {
                LegionError::InvalidValue(format!("Invalid effect mode '{}'. Expected breath, wave, static, off", mode))
            })?;

            let rgb = lighting::RgbColor::from_hex(color).ok_or_else(|| {
                LegionError::InvalidValue(format!("Invalid color '{}'", color))
            })?;

            if zone.to_lowercase() == "all" {
                lighting::set_all_zones(effect, *speed, *brightness, rgb)?;
            } else {
                let z: u8 = zone.parse().map_err(|_| {
                    LegionError::InvalidValue(format!("Invalid zone '{}'", zone))
                })?;
                lighting::set_zone_color(z, effect, *speed, *brightness, rgb)?;
                lighting::commit_rgb_profile(1, *brightness)?;
            }

            if json {
                let resp = serde_json::json!({
                    "success": true,
                    "action": "lighting_effect",
                    "mode": mode,
                    "zone": zone,
                    "color": color,
                    "speed": speed,
                    "brightness": brightness
                });
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else {
                println!(
                    "{} Set effect '{}' on zone {} (speed: {}, brightness: {})",
                    "[OK]".green().bold(),
                    mode.cyan(),
                    zone,
                    speed,
                    brightness
                );
            }
        }
        Some(LightingAction::Off) => {
            lighting::apply_preset("off")?;
            if json {
                let resp = serde_json::json!({
                    "success": true,
                    "action": "lighting_off"
                });
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else {
                println!("{} Keyboard RGB lighting turned off.", "[OK]".green().bold());
            }
        }
        Some(LightingAction::Preset { name }) => {
            lighting::apply_preset(name)?;
            if json {
                let resp = serde_json::json!({
                    "success": true,
                    "action": "lighting_preset",
                    "preset": name
                });
                println!("{}", serde_json::to_string_pretty(&resp).unwrap());
            } else {
                println!("{} Applied RGB lighting preset '{}'", "[OK]".green().bold(), name.cyan());
            }
        }
    }
    Ok(())
}
