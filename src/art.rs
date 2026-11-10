//! omatrimio's artwork, drawn the way Omarchy draws its logo: square pixels
//! and stepped corners. The mark is a ring, the "o" of Omarchy, around a play
//! button, Stremio's job: the two married, hence the name.
//!
//! Every piece is built from the current theme's colors, so the logo, the
//! avatar, and the empty-state art switch with the theme like the rest of
//! the app. The app icon is the one fixed piece, in Omarchy's yellow.

use std::fmt::Write;

use crate::omarchy::Rgb;

/// The mark's drawing in a 512 grid: the ring spans 136..376.
const RING_OUTER: (u32, u32, u32, u32) = (136, 136, 376, 376);
const RING_INNER: (u32, u32, u32, u32) = (176, 176, 336, 336);
const PIXEL: u32 = 20;

/// A rectangle whose corners step in by one pixel, like the letters of the
/// Omarchy logo.
fn stepped((x0, y0, x1, y1): (u32, u32, u32, u32), s: u32) -> String {
    format!(
        "M{a} {y0}H{b}V{c}H{x1}V{d}H{b}V{y1}H{a}V{d}H{x0}V{c}H{a}Z",
        a = x0 + s,
        b = x1 - s,
        c = y0 + s,
        d = y1 - s,
    )
}

/// A right-pointing play button as columns of pixels, 120 tall at its back.
fn play_pixels(left: u32) -> String {
    let mut path = String::new();
    for (column, height) in [120, 100, 80, 60, 40, 20].into_iter().enumerate() {
        let x = left + column as u32 * PIXEL;
        let y = 256 - height / 2;
        let _ = write!(path, "M{x} {y}h{PIXEL}v{height}h-{PIXEL}Z");
    }
    path
}

fn mark_paths(ring: &str, play: &str) -> String {
    format!(
        "<path fill-rule='evenodd' fill='{ring}' d='{outer}{inner}'/><path fill='{play}' d='{pixels}'/>",
        outer = stepped(RING_OUTER, PIXEL),
        inner = stepped(RING_INNER, PIXEL),
        pixels = play_pixels(196),
    )
}

/// The mark alone, cropped to the ring, for the top bar.
pub fn mark(ring: Rgb, play: Rgb) -> String {
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='136 136 240 240'>{}</svg>",
        mark_paths(&ring.to_string(), &play.to_string())
    )
}

/// The mark, faint and with room around it, for empty and error screens.
pub fn empty(color: Rgb) -> String {
    let color = color.to_string();
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='76 76 360 360'><g opacity='0.22'>{}</g></svg>",
        mark_paths(&color, &color)
    )
}

/// The mark beside the name, for the sign-in screen.
pub fn wordmark(ring: Rgb, play: Rgb, text: Rgb, font: &str) -> String {
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='136 136 1000 240'>{mark}\
         <text x='430' y='256' dominant-baseline='central' font-family=\"{font}, monospace\" \
         font-size='150' font-weight='700' letter-spacing='-4' fill='{text}'>omatrimio</text></svg>",
        mark = mark_paths(&ring.to_string(), &play.to_string()),
        font = crate::omarchy::safe_font_name(font),
    )
}

/// A pixel person on the accent, in place of Stremio's cat.
pub fn avatar(background: Rgb, figure: Rgb) -> String {
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'>\
         <rect width='32' height='32' fill='{background}'/>\
         <path fill='{figure}' d='M13 7h6v1h1v6h-1v1h-6v-1h-1V8h1ZM11 18h10v1h2v1h1v5H8v-5h1v-1h2Z'/></svg>"
    )
}

/// The app icon: Omarchy's dark tile with the mark in Omarchy yellow, as
/// Omacut and the other Omarchy apps draw theirs.
pub fn app_icon() -> String {
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='512' height='512' viewBox='0 0 512 512'>\
         <title>omatrimio</title>\
         <defs>\
         <linearGradient id='tile' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='#2c2c2f'/><stop offset='1' stop-color='#161618'/></linearGradient>\
         <linearGradient id='ring' x1='0' y1='0' x2='0' y2='1'><stop offset='0' stop-color='#FFE45C'/><stop offset='1' stop-color='#FFD60A'/></linearGradient>\
         </defs>\
         <rect x='16' y='16' width='480' height='480' rx='112' fill='url(#tile)'/>\
         <g transform='translate(256 256) scale(1.2) translate(-256 -256)'>{}</g></svg>",
        mark_paths("url(#ring)", "#f4f4f5")
    )
}

/// A CSS `url()` holding an SVG, escaped for a data URI.
pub fn css_url(svg: &str) -> String {
    let mut escaped = String::with_capacity(svg.len() + 32);
    for c in svg.chars() {
        match c {
            '%' => escaped.push_str("%25"),
            '#' => escaped.push_str("%23"),
            '<' => escaped.push_str("%3C"),
            '>' => escaped.push_str("%3E"),
            '"' => escaped.push_str("%22"),
            '\n' => {}
            c => escaped.push(c),
        }
    }
    format!("url(\"data:image/svg+xml,{escaped}\")")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepped_corners_close_the_shape() {
        assert_eq!(
            stepped((0, 0, 100, 100), 10),
            "M10 0H90V10H100V90H90V100H10V90H0V10H10Z"
        );
    }

    #[test]
    fn play_fits_inside_the_ring() {
        // Six columns from x=196 end at 316, inside the ring's 176..336 hole.
        let pixels = play_pixels(196);
        assert!(pixels.starts_with("M196 196h20v120"));
        assert!(pixels.ends_with("M296 246h20v20h-20Z"));
    }

    #[test]
    fn data_uri_escapes_what_css_and_urls_need() {
        let url = css_url("<svg fill='#fff'>\"%</svg>");
        assert_eq!(url, "url(\"data:image/svg+xml,%3Csvg fill='%23fff'%3E%22%25%3C/svg%3E\")");
    }

    #[test]
    fn wordmark_font_cannot_break_out() {
        let svg = wordmark(Rgb(0, 0, 0), Rgb(1, 1, 1), Rgb(2, 2, 2), "Evil\"/><script>");
        assert!(!svg.contains("<script>"));
    }

    /// data/omatrimio.svg is what gets installed; it must match the code.
    /// `OMATRIMIO_WRITE_ICON=1 cargo test app_icon_file` rewrites it.
    #[test]
    fn app_icon_file_is_current() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/data/omatrimio.svg");
        let icon = format!("{}\n", app_icon());
        if std::env::var_os("OMATRIMIO_WRITE_ICON").is_some() {
            std::fs::write(path, &icon).unwrap();
        }
        assert_eq!(std::fs::read_to_string(path).unwrap_or_default(), icon);
    }

    #[test]
    fn app_icon_is_a_standalone_svg() {
        let icon = app_icon();
        assert!(icon.starts_with("<svg xmlns='http://www.w3.org/2000/svg' width='512'"));
        assert!(icon.contains("#FFD60A"));
    }
}
