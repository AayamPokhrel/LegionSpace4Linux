use eframe::egui::{Color32, Rounding, Stroke};

// ==============================================================================
// Google Material 3 Expressive (M3E) / Material You Dark Theme Palette
// Authentic HCT (Hue, Chroma, Tone) Token Architecture
// ==============================================================================

// Base Tonal Surfaces (Rich Warm Violet-Black, NO pure harsh blacks)
pub const SURFACE_DIM: Color32 = Color32::from_rgb(17, 15, 23); // #110F17 Deepest Background
pub const SURFACE: Color32 = Color32::from_rgb(21, 19, 29); // #15131D Base Canvas
pub const SURFACE_CONTAINER_LOW: Color32 = Color32::from_rgb(28, 25, 37); // #1C1925 Nav Rail & Header
pub const SURFACE_CONTAINER: Color32 = Color32::from_rgb(35, 32, 47); // #23202F Standard Cards
pub const SURFACE_CONTAINER_HIGH: Color32 = Color32::from_rgb(46, 42, 61); // #2E2A3D Elevated Surfaces
pub const SURFACE_CONTAINER_HIGHEST: Color32 = Color32::from_rgb(58, 53, 77); // #3A354D Interactive Tracks & Chips

// Dynamic Material 3 Expressive Accent Roles (Soft Lavender / Violet / Rose)
pub const PRIMARY: Color32 = Color32::from_rgb(208, 188, 255); // #D0BCFF Radiant Lavender
pub const ON_PRIMARY: Color32 = Color32::from_rgb(56, 30, 114); // #381E72 Deep Royal Violet
pub const PRIMARY_CONTAINER: Color32 = Color32::from_rgb(79, 55, 139); // #4F378B Active Selection Fill
pub const ON_PRIMARY_CONTAINER: Color32 = Color32::from_rgb(234, 221, 255); // #EADDFF High-Contrast Lavender

pub const SECONDARY: Color32 = Color32::from_rgb(204, 194, 220); // #CCC2DC Soft Mauve
#[allow(dead_code)]
pub const SECONDARY_CONTAINER: Color32 = Color32::from_rgb(74, 68, 88); // #4A4458
#[allow(dead_code)]
pub const ON_SECONDARY_CONTAINER: Color32 = Color32::from_rgb(232, 222, 248); // #E8DEF8

#[allow(dead_code)]
pub const TERTIARY: Color32 = Color32::from_rgb(239, 184, 200); // #EFB8C8 Warm Dusty Rose
#[allow(dead_code)]
pub const TERTIARY_CONTAINER: Color32 = Color32::from_rgb(99, 59, 72); // #633B48
#[allow(dead_code)]
pub const ON_TERTIARY_CONTAINER: Color32 = Color32::from_rgb(255, 216, 228); // #FFD8E4

// Crisp Legible Typography Tokens (Strict WCAG AAA Compliance on Dark Surfaces)
pub const ON_SURFACE: Color32 = Color32::from_rgb(245, 240, 248); // #F5F0F8 Crisp Headline White (95% Luminance)
pub const ON_SURFACE_VARIANT: Color32 = Color32::from_rgb(202, 196, 208); // #CAC4D0 Body & Subtitle Text (80% Luminance)
pub const OUTLINE_MUTED: Color32 = Color32::from_rgb(150, 144, 158); // #96909E Captions & Secondary (60% Luminance)

// Hardware Semantic Accents
pub const COLOR_QUIET: Color32 = Color32::from_rgb(138, 201, 255); // #8AC9FF M3 Soft Sky Blue
pub const COLOR_BALANCED: Color32 = Color32::from_rgb(245, 240, 248); // Crisp White
pub const COLOR_PERF: Color32 = Color32::from_rgb(255, 137, 137); // #FF8989 M3 Coral Red
pub const COLOR_CUSTOM: Color32 = Color32::from_rgb(222, 158, 255); // #DE9EFF M3 Electric Orchid
pub const COLOR_SUCCESS: Color32 = Color32::from_rgb(152, 242, 178); // #98F2B2 M3 Mint Green
pub const COLOR_WARNING: Color32 = Color32::from_rgb(255, 214, 125); // #FFD67D M3 Amber Gold
pub const COLOR_TEAL: Color32 = Color32::from_rgb(128, 222, 215); // #80DED7 M3 Soft Teal

// Expressive Shape Radii Tokens
pub const ROUNDING_SQUIRCLE_LG: Rounding = Rounding {
    nw: 24.0,
    ne: 24.0,
    sw: 24.0,
    se: 24.0,
};

pub const ROUNDING_SQUIRCLE_XL: Rounding = Rounding {
    nw: 32.0,
    ne: 32.0,
    sw: 32.0,
    se: 32.0,
};

pub const ROUNDING_STADIUM: Rounding = Rounding {
    nw: 9999.0,
    ne: 9999.0,
    sw: 9999.0,
    se: 9999.0,
};

// Zero Borders Token (Pure Tonal Elevation)
#[allow(dead_code)]
pub const STROKE_NONE: Stroke = Stroke::NONE;
