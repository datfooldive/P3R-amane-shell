use amane::{Color, Text, Weight};

// Adwaita Sans ships a Black weight, which the headings need
pub const DISPLAY: &str = "Adwaita Sans";
pub const ICONS: &str = "MesloLGS Nerd Font Propo";

pub const NAVY: &str = "#040b26";
pub const DEEP: &str = "#0a2263";
pub const BLUE: &str = "#1550d6";
pub const CYAN: &str = "#36d4ff";
pub const AQUA: &str = "#a6f2ff";
pub const INK: &str = "#05112e";
pub const RED: &str = "#ff2e55";

pub const DH_DEEP: &str = "#06200f";
pub const DH_GREEN: &str = "#3dff86";
pub const DH_MOON: &str = "#f2ffb3";
pub const BLOOD: &str = "#b3001e";

pub fn color(hex: &str) -> Color {
    Color::from(hex)
}

pub fn with_alpha(hex: &str, alpha: u8) -> Color {
    let base = Color::from(hex);
    Color::rgba(base.red(), base.green(), base.blue(), alpha)
}

pub fn heading(content: impl Into<String>, size: f32, hex: &str) -> Text {
    Text::new(content).font(DISPLAY).weight(Weight::Black).size(size).color(color(hex))
}

pub fn label(content: impl Into<String>, size: f32, hex: &str) -> Text {
    Text::new(content).font(DISPLAY).weight(Weight::Bold).size(size).color(color(hex))
}

pub fn icon(glyph: &str, size: f32, hex: &str) -> Text {
    Text::new(glyph).font(ICONS).size(size).color(color(hex)).tight()
}

pub fn accent(dark_hour: bool) -> &'static str {
    if dark_hour { DH_GREEN } else { CYAN }
}
