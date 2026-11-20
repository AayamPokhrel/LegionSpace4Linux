use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "legion-cli",
    author = "AayamPokhrel <pokhrelaayam1010@gmail.com>",
    version = env!("CARGO_PKG_VERSION"),
    about = "Native Linux hardware control utility for Lenovo Legion laptops (Pro 5 16ADR10)",
    long_about = "A high-performance native Linux userspace driver and CLI utility providing \
direct hardware control for Lenovo Legion laptops, reverse-engineered from vendor software."
)]
pub struct Cli {
    #[arg(
        short,
        long,
        global = true,
        help = "Output result in JSON format for scripting and integration"
    )]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(
        about = "Probe hardware, detect capabilities, and verify Legion laptop subsystem support"
    )]
    Probe,

    #[command(about = "Display comprehensive hardware status overview (default action)")]
    Status,

    #[command(
        alias = "thermal",
        about = "Manage thermal power profiles and power LED indicator"
    )]
    Profile(ProfileCmd),

    #[command(about = "Monitor and control cooling fan speeds")]
    Fan(FanCmd),

    #[command(about = "Manage battery conservation mode and power telemetry")]
    Battery(BatteryCmd),

    #[command(about = "Fine-tune custom mode firmware power limits and temperature thresholds")]
    Custom(CustomCmd),

    #[command(about = "Manage NVIDIA discrete GPU telemetry, TGP limits, and clocks")]
    Gpu(GpuCmd),

    #[command(about = "Manage hardware camera e-shutter privacy switch")]
    Camera(CameraCmd),

    #[command(about = "Manage OLED care, anti-burn protection, pixel shift, and display overdrive")]
    Oled(OledCmd),

    #[command(about = "Manage Fn-Lock key mode (F1-F12 vs media hotkeys)")]
    FnLock(FnLockCmd),

    #[command(
        alias = "rgb",
        about = "Control 4-zone RGB lighting, ACPI backlight, and Fn-Lock"
    )]
    Lighting(LightingCmd),
}

// ----------------------------------------------------------------------------
// Thermal / Profile Subcommands
// ----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct ProfileCmd {
    #[command(subcommand)]
    pub action: Option<ProfileAction>,
}

#[derive(Subcommand, Debug)]
pub enum ProfileAction {
    #[command(about = "Get active thermal profile and power button LED color")]
    Get,

    #[command(about = "Set thermal profile (quiet, balanced, performance, max-power, custom)")]
    Set {
        #[arg(help = "Target profile name (e.g. quiet, balanced, performance, max-power, custom)")]
        name: String,
    },

    #[command(about = "List supported thermal profiles for this hardware")]
    List,
}

// ----------------------------------------------------------------------------
// Fan Subcommands
// ----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct FanCmd {
    #[command(subcommand)]
    pub action: Option<FanAction>,
}

#[derive(Subcommand, Debug)]
pub enum FanAction {
    #[command(about = "Get current fan speeds (RPM), bounds, and mode")]
    Status,

    #[command(about = "Set target fan speed in RPM (1700-5100)")]
    Set {
        #[arg(long, help = "Target RPM for Fan 1 (CPU)")]
        fan1: Option<u32>,

        #[arg(long, help = "Target RPM for Fan 2 (GPU)")]
        fan2: Option<u32>,

        #[arg(long, help = "Set both Fan 1 and Fan 2 to the same RPM")]
        both: Option<u32>,
    },

    #[command(about = "Reset fans to firmware-managed automatic thermal curve")]
    Auto,

    #[command(alias = "max", about = "Set fans to maximum speed (5100 RPM)")]
    Full,
}

// ----------------------------------------------------------------------------
// Battery Subcommands
// ----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct BatteryCmd {
    #[command(subcommand)]
    pub action: Option<BatteryAction>,
}

#[derive(Subcommand, Debug)]
pub enum BatteryAction {
    #[command(about = "Get battery health, percentage, charge rate, and conservation status")]
    Status,

    #[command(about = "Toggle battery conservation mode (caps charging at 80% to preserve lifespan)")]
    Conservation {
        #[arg(help = "State: on/enable/1 or off/disable/0")]
        state: Option<ToggleState>,
    },

    #[command(about = "Toggle always-on USB charging while laptop is off/sleeping")]
    UsbCharge {
        #[arg(help = "State: on/enable/1 or off/disable/0")]
        state: Option<ToggleState>,
    },

    #[command(about = "Toggle battery rapid charging mode (fast charging 0-70% in 30 mins)")]
    RapidCharge {
        #[arg(help = "State: on/enable/1 or off/disable/0")]
        state: Option<ToggleState>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ToggleState {
    On,
    Off,
    Enable,
    Disable,
}

impl ToggleState {
    pub fn is_enabled(&self) -> bool {
        matches!(self, ToggleState::On | ToggleState::Enable)
    }
}

// ----------------------------------------------------------------------------
// Custom Mode Firmware Attributes Subcommands
// ----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct CustomCmd {
    #[command(subcommand)]
    pub action: Option<CustomAction>,
}

#[derive(Subcommand, Debug)]
pub enum CustomAction {
    #[command(about = "List all firmware attributes with current values and safe bounds")]
    Status,

    #[command(about = "Set custom firmware power limits and temperature thresholds")]
    Set {
        #[arg(long, help = "Firmware attribute name (e.g. ppt_pl1_spl)")]
        name: Option<String>,

        #[arg(long, help = "Attribute value to write")]
        value: Option<i64>,

        #[arg(long, help = "CPU Sustained Power Limit PL1 (Watts)")]
        pl1: Option<i64>,

        #[arg(long, help = "CPU Short-Term Power Limit PL2 (Watts)")]
        pl2: Option<i64>,

        #[arg(long, help = "CPU Fast Peak Power Limit PL3 / FPPT (Watts)")]
        pl3: Option<i64>,

        #[arg(long, help = "CPU Cross Loading Power Limit (Watts)")]
        cpu_cl: Option<i64>,

        #[arg(long, help = "CPU Temperature Limit (°C)")]
        cpu_temp: Option<i64>,

        #[arg(long, help = "GPU Temperature Limit (°C)")]
        gpu_temp: Option<i64>,

        #[arg(long, help = "GPU AC processing power baseline offset (Watts)")]
        nv_offset: Option<i64>,

        #[arg(long, help = "GPU Configurable TGP (Watts)")]
        ctgp: Option<i64>,

        #[arg(long, help = "GPU Dynamic Boost (PPAB) limit (Watts)")]
        ppab: Option<i64>,

        #[arg(long, help = "GPU to CPU Dynamic Boost limit (Watts)")]
        cpu_boost: Option<i64>,

        #[arg(long, help = "Legacy alias for nv_offset (Watts)")]
        tgp_offset: Option<i64>,
    },
}

// ----------------------------------------------------------------------------
// GPU Subcommands
// ----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct GpuCmd {
    #[command(subcommand)]
    pub action: Option<GpuAction>,
}

#[derive(Subcommand, Debug)]
pub enum GpuAction {
    #[command(about = "Display comprehensive GPU telemetry, clocks, TGP, and thermals")]
    Status,

    #[command(about = "Tune GPU power limit and firmware parameters")]
    Set {
        #[arg(long, help = "GPU Power Limit / TGP in Watts (5.0 - 115.0 W)")]
        power_limit: Option<f32>,

        #[arg(long, help = "GPU Thermal Throttle Ceiling (°C)")]
        gpu_temp: Option<i64>,

        #[arg(long, help = "GPU AC processing power baseline offset in Watts (10 - 85 W)")]
        ac_offset: Option<i64>,

        #[arg(long, help = "Configurable TGP in Watts")]
        ctgp: Option<i64>,

        #[arg(long, help = "Dynamic Boost limit (PPAB) in Watts")]
        ppab: Option<i64>,

        #[arg(long, help = "GPU to CPU Dynamic Boost in Watts")]
        cpu_boost: Option<i64>,
    },

    #[command(about = "Lock GPU graphics core clocks (min_mhz,max_mhz)")]
    LockClocks {
        #[arg(help = "Minimum frequency in MHz")]
        min: u32,
        #[arg(help = "Maximum frequency in MHz")]
        max: u32,
    },

    #[command(about = "Reset GPU graphics core clocks to automatic driver management")]
    ResetClocks,

    #[command(about = "Get or set GPU working mode (hybrid, dgpu, igpu, auto)")]
    Mode {
        #[arg(help = "Target mode: hybrid (Optimus), dgpu (Discrete only), igpu (Eco/Integrated), auto (Hybrid-Auto)")]
        mode: Option<String>,
    },

    #[command(about = "Get or set NVIDIA GPUPowerMizerMode (0 = Adaptive, 1 = Prefer Maximum Performance, 2 = Auto)")]
    Powermizer {
        #[arg(help = "Mode level (0 = Adaptive, 1 = Max Performance, 2 = Auto)")]
        level: Option<u32>,
    },

    #[command(about = "List applications and processes currently utilizing the discrete NVIDIA GPU")]
    Apps,
}

// ----------------------------------------------------------------------------
// Fn-Lock Subcommands
// ----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct FnLockCmd {
    #[arg(help = "State: on/enable/1 or off/disable/0")]
    pub state: Option<ToggleState>,
}

// ----------------------------------------------------------------------------
// OLED & Display Care Subcommands
// ----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct OledCmd {
    #[command(subcommand)]
    pub action: Option<OledAction>,
}

#[derive(Subcommand, Debug)]
pub enum OledAction {
    #[command(about = "Display OLED care, pixel shift, and anti-burn protection status")]
    Status,

    #[command(about = "Configure OLED care settings")]
    Set {
        #[arg(long, help = "Master OLED anti-burn protection (on/off)")]
        master: Option<ToggleState>,

        #[arg(long, help = "Pixel shift periodic micro-translations (on/off)")]
        pixel_shift: Option<ToggleState>,

        #[arg(long, help = "Static taskbar dimming protection (on/off)")]
        taskbar_dim: Option<ToggleState>,

        #[arg(long, help = "Default system dark mode (on/off)")]
        dark_mode: Option<ToggleState>,

        #[arg(long, help = "LCD panel overdrive / response time accelerator (on/off)")]
        overdrive: Option<ToggleState>,

        #[arg(long, help = "Adaptive refresh rate (Fn+R dynamic 60Hz/165Hz) (on/off)")]
        adaptive_rr: Option<ToggleState>,
    },

    #[command(about = "Trigger or reset panel pixel clean cycle tracker (4-hour maintenance)")]
    Clean,
}

// ----------------------------------------------------------------------------
// Camera Subcommands
// ----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct CameraCmd {
    #[arg(help = "State: on/enable/1 or off/disable/0")]
    pub state: Option<ToggleState>,
}

// ----------------------------------------------------------------------------
// Lighting / RGB Subcommands
// ----------------------------------------------------------------------------
#[derive(Args, Debug)]
pub struct LightingCmd {
    #[command(subcommand)]
    pub action: Option<LightingAction>,
}

#[derive(Subcommand, Debug)]
pub enum LightingAction {
    #[command(about = "Show lighting status, ACPI backlight level, and RGB controller info")]
    Status,

    #[command(about = "Set ACPI keyboard backlight brightness level (0 = off, 1 = low, 2 = high)")]
    KbdBacklight {
        #[arg(help = "Backlight level: 0, 1, or 2")]
        level: u32,
    },

    #[command(about = "Toggle Fn-Lock state")]
    FnLock {
        #[arg(help = "State: on/enable/1 or off/disable/0")]
        state: ToggleState,
    },

    #[command(about = "Set static solid color on 4-zone RGB keyboard")]
    Static {
        #[arg(
            short,
            long,
            default_value = "all",
            help = "Zone: 1, 2, 3, 4, or all"
        )]
        zone: String,

        #[arg(
            short,
            long,
            default_value = "cyan",
            help = "Color: hex code (#00d3fe) or name (cyan, red, green, blue, white, yellow, orange, purple)"
        )]
        color: String,

        #[arg(
            short,
            long,
            default_value = "3",
            help = "Brightness: 1 (low), 2 (medium), 3 (high)"
        )]
        brightness: u8,
    },

    #[command(about = "Set dynamic lighting effect on 4-zone RGB keyboard")]
    Effect {
        #[arg(
            short,
            long,
            default_value = "wave",
            help = "Effect: breath, wave, static, off"
        )]
        mode: String,

        #[arg(
            short,
            long,
            default_value = "all",
            help = "Zone: 1, 2, 3, 4, or all"
        )]
        zone: String,

        #[arg(
            short,
            long,
            default_value = "cyan",
            help = "Color: hex code or name"
        )]
        color: String,

        #[arg(short, long, default_value = "2", help = "Speed: 1 (slow) to 4 (fast)")]
        speed: u8,

        #[arg(short, long, default_value = "3", help = "Brightness: 1 (low) to 3 (high)")]
        brightness: u8,
    },

    #[command(about = "Turn off all 4 RGB keyboard zones")]
    Off,

    #[command(about = "Apply predefined RGB lighting preset")]
    Preset {
        #[arg(
            help = "Preset name: legion-blue, crimson, stealth, cyberpunk, spectrum"
        )]
        name: String,
    },
}
