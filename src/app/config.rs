pub const APP_ID: &str = match cfg!(debug_assertions) {
    true => "io.github.pakvothe.omatrimio.Devel",
    false => "io.github.pakvothe.omatrimio",
};
pub const APP_NAME: &str = "omatrimio";
// Addon sites hand out stremio:// links; opening them is part of being
// compatible with Stremio addons.
pub const URI_SCHEME: &str = "stremio://";
pub const STYLE: &str = include_str!("style.css");
