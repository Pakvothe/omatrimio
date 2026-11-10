//! The Omarchy look: the current theme's colors, Hyprland's window rounding,
//! and the desktop font, turned into a stylesheet for Stremio's web UI.
//!
//! stremio-web takes its whole palette from CSS custom properties on `:root`
//! (src/App/styles.less), so overriding those restyles the app without
//! touching its code, and keeps working as web.stremio.com updates.
//!
//! Only variables are overridden, never `body`'s background: the video is
//! drawn underneath the web view, and the player clears that background to
//! show it. The page's gradient runs between the two background variables,
//! so setting both to the theme background is what makes it flat.
//!
//! Omarchy apps use one accent, so stremio-web's secondary accent, its green
//! "action" color for play and install, becomes the theme accent too.

use std::{collections::HashMap, fmt, fs, path::PathBuf, process::Command};

use crate::art;

/// The layout over stremio-web: side bar, top bar, and artwork.
const LAYOUT: &str = include_str!("omarchy.css");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn parse(text: &str) -> Option<Self> {
        let hex = text.trim().strip_prefix('#')?;
        if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        Some(Self(channel(0)?, channel(2)?, channel(4)?))
    }

    fn alpha(self, alpha: f32) -> String {
        format!("rgba({}, {}, {}, {alpha})", self.0, self.1, self.2)
    }

    /// WCAG relative luminance.
    fn luminance(self) -> f64 {
        let linear = |c: u8| {
            let v = f64::from(c) / 255.0;
            if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * linear(self.0) + 0.7152 * linear(self.1) + 0.0722 * linear(self.2)
    }

    /// WCAG contrast ratio between two colors, from 1 to 21.
    fn contrast(self, other: Rgb) -> f64 {
        let (a, b) = (self.luminance(), other.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// The color for text on this color. The theme's own background or
    /// foreground when one of them reads well (4.5:1, WCAG's bar for body
    /// text), so text on an accent stays in the theme's palette; otherwise
    /// whichever of those and pure black and white reads best. Rose Pine's
    /// accent, for one, is too close to both of its text colors.
    fn readable_text(self, palette: &Palette) -> Rgb {
        let best = |colors: &[Rgb]| {
            colors
                .iter()
                .copied()
                .max_by(|a, b| self.contrast(*a).total_cmp(&self.contrast(*b)))
                .unwrap_or(Rgb(0, 0, 0))
        };
        let themed = best(&[palette.background, palette.foreground]);
        if self.contrast(themed) >= 4.5 {
            themed
        } else {
            best(&[palette.background, palette.foreground, Rgb(0, 0, 0), Rgb(255, 255, 255)])
        }
    }

    fn mix(self, other: Rgb, amount: f32) -> Rgb {
        let blend = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * amount).round() as u8;
        Rgb(blend(self.0, other.0), blend(self.1, other.1), blend(self.2, other.2))
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub light: bool,
    pub accent: Rgb,
    pub background: Rgb,
    pub dark_background: Rgb,
    pub lighter_background: Rgb,
    pub foreground: Rgb,
    pub green: Rgb,
    pub yellow: Rgb,
    pub blue: Rgb,
    pub orange: Rgb,
    pub red: Rgb,
}

impl Default for Palette {
    /// Omarchy's default look, for when no theme is installed.
    fn default() -> Self {
        Self::from_colors(&HashMap::new())
    }
}

impl Palette {
    /// Older and hand-made themes leave keys out, so each one falls back to
    /// something derived from the keys a theme always has.
    pub fn from_colors(colors: &HashMap<String, String>) -> Self {
        let get = |key: &str| colors.get(key).and_then(|value| Rgb::parse(value));
        let light = colors.get("mode").is_some_and(|mode| mode == "light");

        let background = get("background").unwrap_or(if light { Rgb(250, 250, 250) } else { Rgb(18, 18, 18) });
        let foreground = get("foreground").unwrap_or(if light { Rgb(30, 30, 30) } else { Rgb(230, 230, 230) });
        let accent = get("accent").or_else(|| get("color4")).unwrap_or(Rgb(0xff, 0xd6, 0x0a));

        Self {
            light,
            accent,
            background,
            dark_background: get("dark_background").unwrap_or(background.mix(Rgb(0, 0, 0), 0.25)),
            lighter_background: get("lighter_background").unwrap_or(background.mix(foreground, 0.08)),
            foreground,
            green: get("green").or_else(|| get("color2")).unwrap_or(Rgb(0x22, 0xb3, 0x65)),
            yellow: get("yellow").or_else(|| get("color3")).unwrap_or(Rgb(0xf6, 0xc7, 0x00)),
            blue: get("blue").or_else(|| get("color4")).unwrap_or(Rgb(0x12, 0x45, 0xa6)),
            orange: get("orange").or_else(|| get("color3")).unwrap_or(Rgb(0xff, 0xa5, 0x00)),
            red: get("red").or_else(|| get("color1")).unwrap_or(Rgb(0xdc, 0x26, 0x26)),
        }
    }
}

/// `key = "value"` lines of colors.toml. Omarchy's file is flat, so a TOML
/// parser would only add a dependency.
pub fn parse_colors(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            let value = value.trim();
            // Values are quoted and start with '#', so a trailing comment
            // only begins after the closing quote.
            let value = match value.chars().next() {
                Some(quote @ ('"' | '\'')) => value[1..].split(quote).next()?,
                _ => value.split(|c: char| c.is_whitespace() || c == '#').next()?,
            };
            Some((key.trim().to_string(), value.to_string()))
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Look {
    pub palette: Palette,
    pub radius: u32,
    /// Hyprland's window border width, which focused cards borrow.
    pub border: u32,
    pub font: Option<String>,
}

impl Look {
    pub fn stylesheet(&self) -> String {
        let p = &self.palette;
        let font = self
            .font
            .as_deref()
            .map(|font| format!("\"{}\", ", safe_font_name(font)))
            .unwrap_or_default();

        format!(
            r#":root {{
    --primary-background-color: {background} !important;
    --secondary-background-color: {background} !important;
    --modal-background-color: {dark} !important;
    --primary-foreground-color: {foreground} !important;
    --secondary-foreground-color: {on_accent} !important;
    --on-primary-accent-color: {on_accent} !important;
    --on-secondary-accent-color: {on_accent} !important;
    --on-tertiary-accent-color: {on_yellow} !important;
    --on-quaternary-accent-color: {on_blue} !important;
    --on-warning-accent-color: {on_orange} !important;
    --on-danger-accent-color: {on_red} !important;
    --primary-accent-color: {accent} !important;
    --secondary-accent-color: {accent} !important;
    --omatrimio-logo: {art_logo};
    --omatrimio-wordmark: {art_wordmark};
    --omatrimio-empty: {art_empty};
    --omatrimio-avatar: {art_avatar};
    --tertiary-accent-color: {yellow} !important;
    --quaternary-accent-color: {blue} !important;
    --warning-accent-color: {orange} !important;
    --danger-accent-color: {red} !important;
    --overlay-color: {overlay} !important;
    --outer-glow: 0px 0px 15px {glow} !important;
    --border-radius: {radius}px !important;
    --omatrimio-border: {border}px;
    --omatrimio-credit: "omatrimio {version} · based on Stremio";
    color-scheme: {scheme};
}}

body, body * {{
    font-family: {font}"TwemojiFlags", sans-serif !important;
}}

svg[data-omatrimio-icon] > path:not([data-omatrimio-glyph]) {{
    display: none !important;
}}

svg[data-omatrimio-icon] > path[data-omatrimio-glyph] {{
    fill: currentColor !important;
    stroke: none !important;
}}

::selection {{
    background-color: {selection} !important;
}}
"#,
            background = p.background,
            dark = p.dark_background,
            foreground = p.foreground,
            on_accent = p.accent.readable_text(p),
            on_yellow = p.yellow.readable_text(p),
            on_blue = p.blue.readable_text(p),
            on_orange = p.orange.readable_text(p),
            on_red = p.red.readable_text(p),
            accent = p.accent,
            yellow = p.yellow,
            blue = p.blue,
            orange = p.orange,
            red = p.red,
            overlay = p.foreground.alpha(0.06),
            glow = p.accent.alpha(0.37),
            radius = self.radius,
            border = self.border.clamp(1, 4),
            version = env!("CARGO_PKG_VERSION"),
            scheme = if p.light { "light" } else { "dark" },
            selection = p.accent.alpha(0.35),
            art_logo = art::css_url(&art::mark(p.accent, p.foreground)),
            art_wordmark = art::css_url(&art::wordmark(
                p.accent,
                p.foreground,
                p.foreground,
                self.font.as_deref().unwrap_or("monospace"),
            )),
            art_empty = art::css_url(&art::empty(p.foreground)),
            art_avatar = art::css_url(&art::avatar(p.accent, p.accent.readable_text(p))),
        ) + &self.backdrop_palette() + LAYOUT
    }
}

impl Look {
    /// The title page, its preview panel in Discover, and the player put text
    /// over a photo or the video, so they stay dark the way players do. On a light theme the palette is
    /// swapped inside them: the theme's dark text color becomes the surface
    /// and its light background becomes the text. Dark themes need nothing.
    fn backdrop_palette(&self) -> String {
        let p = &self.palette;
        if !p.light {
            return String::new();
        }
        let (surface, text) = (p.foreground, p.background);
        format!(
            r#"
[class*="metadetails-container-"],
[class*="meta-preview-container-"],
[class*="player-container-"] {{
    --primary-background-color: {surface} !important;
    --secondary-background-color: {surface} !important;
    --modal-background-color: {deep} !important;
    --primary-foreground-color: {text} !important;
    --overlay-color: {overlay} !important;
    color: {text} !important;
    color-scheme: dark;
}}
"#,
            deep = surface.mix(Rgb(0, 0, 0), 0.35),
            overlay = text.alpha(0.08),
        )
    }
}

/// A font family name with only the characters font names use, so it can't
/// close the CSS string or rule (or the SVG attribute) it's placed in.
pub fn safe_font_name(font: &str) -> String {
    font.chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.'))
        .collect()
}

/// Where Omarchy keeps the current theme.
pub fn current_directory() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".local/state/omarchy/current")
}

/// Reads the look the desktop has right now. Every part is optional: without
/// Omarchy, Hyprland, or fontconfig, that part keeps its default.
pub fn read_look() -> Look {
    let colors = fs::read_to_string(current_directory().join("theme/colors.toml"))
        .map(|text| parse_colors(&text))
        .unwrap_or_default();

    Look {
        palette: Palette::from_colors(&colors),
        radius: hyprland_option("decoration:rounding").unwrap_or(0),
        border: hyprland_option("general:border_size").unwrap_or(2),
        font: monospace_font(),
    }
}

/// A Hyprland option's effective value (window rounding, border width), which
/// themes and personal overrides can both set, so ask hyprctl rather than
/// reading its config.
fn hyprland_option(name: &str) -> Option<u32> {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    let output = Command::new("hyprctl")
        .args(["-j", "getoption", name])
        .output()
        .ok()?;
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    json.get("int")?.as_u64().map(|value| value as u32)
}

/// The font Omarchy sets for the desktop (`omarchy font current` reads it the
/// same way: fontconfig's monospace match).
fn monospace_font() -> Option<String> {
    let output = Command::new("fc-match")
        .args(["monospace", "-f", "%{family}"])
        .output()
        .ok()?;
    let family = String::from_utf8_lossy(&output.stdout);
    let first = family.split(',').next()?.trim();
    (!first.is_empty()).then(|| first.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NETRUNNER: &str = r##"mode = "dark"

accent = "#00e5ff"
background = "#05080d" # trailing comment
dark_background = "#04060a"
lighter_background = "#0d1520"
foreground = "#c8f0ff"
hyprland_active_border = "rgba(00e5ffee) rgba(c44dffee) 45deg"
red = "#ff2e63"
"##;

    #[test]
    fn parses_colors_toml() {
        let colors = parse_colors(NETRUNNER);
        assert_eq!(colors["accent"], "#00e5ff");
        assert_eq!(colors["background"], "#05080d");
        assert_eq!(colors["mode"], "dark");
    }

    #[test]
    fn builds_palette_with_fallbacks() {
        let palette = Palette::from_colors(&parse_colors(NETRUNNER));
        assert_eq!(palette.accent, Rgb(0x00, 0xe5, 0xff));
        assert_eq!(palette.red, Rgb(0xff, 0x2e, 0x63));
        assert!(!palette.light);
        // Missing keys fall back rather than failing.
        assert_eq!(palette.green, Rgb(0x22, 0xb3, 0x65));
    }

    #[test]
    fn falls_back_without_omarchy() {
        let palette = Palette::default();
        assert_eq!(palette.accent, Rgb(0xff, 0xd6, 0x0a));
        assert!(!palette.light);
    }

    #[test]
    fn stylesheet_carries_the_look() {
        let look = Look {
            palette: Palette::from_colors(&parse_colors(NETRUNNER)),
            radius: 6,
            border: 2,
            font: Some("JetBrainsMono Nerd Font".into()),
        };
        let css = look.stylesheet();
        assert!(css.contains("--primary-accent-color: #00e5ff !important;"));
        assert!(css.contains("--border-radius: 6px !important;"));
        // Stremio's green action color follows the theme's single accent.
        assert!(css.contains("--secondary-accent-color: #00e5ff !important;"));
        // The theme's dark background as text on the bright cyan accent.
        assert!(css.contains("--secondary-foreground-color: #05080d !important;"));
        assert!(css.contains("font-family: \"JetBrainsMono Nerd Font\", "));
    }

    #[test]
    fn text_on_bright_green_uses_the_dark_background() {
        let palette = Palette::from_colors(&parse_colors(&format!("{NETRUNNER}\ngreen = \"#39ff88\"\n")));
        assert_eq!(palette.green.readable_text(&palette), palette.background);
        // And the light foreground on a dark blue.
        assert_eq!(Rgb(0x12, 0x45, 0xa6).readable_text(&palette), palette.foreground);
    }

    #[test]
    fn text_on_a_weak_accent_falls_back_to_black_or_white() {
        // Rose Pine: neither of its text colors reaches 4.5:1 on the accent.
        let colors = "mode = \"light\"\nbackground = \"#faf4ed\"\nforeground = \"#575279\"\naccent = \"#56949f\"\n";
        let palette = Palette::from_colors(&parse_colors(colors));
        let text = palette.accent.readable_text(&palette);
        assert!(text == Rgb(0, 0, 0) || text == Rgb(255, 255, 255));
        assert!(palette.accent.contrast(text) > palette.accent.contrast(palette.background));
    }

    #[test]
    fn light_themes_keep_backdrop_pages_dark() {
        let latte = "mode = \"light\"\nbackground = \"#eff1f5\"\nforeground = \"#4c4f69\"\naccent = \"#1e66f5\"\n";
        let light = Look { palette: Palette::from_colors(&parse_colors(latte)), ..Look::default() };
        let css = light.stylesheet();
        // Inside the title page, the surface is the theme's dark text color.
        assert!(css.contains("[class*=\"metadetails-container-\"]"));
        assert!(css.contains("--primary-background-color: #4c4f69 !important;\n    --secondary"));
        // A dark theme has no such override.
        assert!(!Look::default().stylesheet().contains("metadetails-container"));
    }

    #[test]
    fn font_names_cannot_break_out_of_the_rule() {
        let look = Look { font: Some("Evil\"; } body { display: none".into()), ..Look::default() };
        // The quote and brace are stripped, so the name can't close the
        // rule and start a new one.
        assert!(!look.stylesheet().contains("} body {"));
        assert!(!look.stylesheet().contains("Evil\";"));
    }
}
