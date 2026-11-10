mod app;
mod art;
mod config;
mod gpu;
mod omarchy;
mod server;
mod utils;

use std::{env, fs, ptr};

use clap::Parser;
use gtk::glib::{ExitCode, object::ObjectExt};
use tokio::runtime::Runtime;
use tracing::error;

use crate::{
    app::Application,
    config::{DATA_DIR, FALLBACK_URL, GETTEXT_DIR_FLATPAK, GETTEXT_DOMAIN, STARTUP_URL},
    server::Server,
};

#[derive(Parser, Debug)]
#[command(version, ignore_errors(true), allow_hyphen_values(true))]
struct Args {
    /// Open dev tools
    #[arg(short, long)]
    dev: bool,
    /// Startup url
    #[arg(short, long, default_value = STARTUP_URL)]
    url: String,
    /// Disable window decorations
    #[arg(short, long)]
    no_window_decorations: bool,

    #[arg(trailing_var_arg = true)]
    trailing: Vec<String>,
}

fn main() -> ExitCode {
    // GTK 4.22 defaults to the Vulkan GSK renderer. Compositing our OpenGL video
    // GLArea through it forces a per-frame GL->Vulkan copy, which dominates
    // playback CPU even when hardware decoding is active. Prefer the GL renderer,
    // unless a renderer was already chosen — data/stremio.sh sets `opengl` for
    // Nvidia, and `opengl` and `gl` are the same renderer here, so the two compose.
    // Must be set before GTK initializes.
    //
    // The name is `gl`: GTK renamed the new GL renderer in 4.18. `ngl` still
    // resolves to it as a deprecated alias, but warns. Any name GTK does not
    // recognize falls back to the Vulkan renderer this is meant to avoid, so the
    // value is worth keeping in step with `GSK_RENDERER=help`.
    //
    // SAFETY: this runs at the very start of main, before any other threads are
    // spawned, so there is no concurrent access to the environment.
    if env::var_os("GSK_RENDERER").is_none() {
        unsafe { env::set_var("GSK_RENDERER", "gl") };
    }

    // On a virtual GPU, WebKit's GPU rasterization makes the board's posters
    // flicker, so draw the web UI on the CPU there. Video still goes through GL.
    // OMATRIMIO_CPU_RENDERING=1 or =0 forces either way.
    if env::var_os("WEBKIT_SKIA_ENABLE_CPU_RENDERING").is_none() {
        let cpu_rendering = match env::var("OMATRIMIO_CPU_RENDERING").as_deref() {
            Ok("1") => true,
            Ok("0") => false,
            _ => gpu::only_virtual_gpus(),
        };
        if cpu_rendering {
            // SAFETY: still before any other thread exists, as above.
            unsafe { env::set_var("WEBKIT_SKIA_ENABLE_CPU_RENDERING", "1") };
        }
    }

    tracing_subscriber::fmt::init();

    let data_dir = dirs::data_dir()
        .expect("Failed to get data dir")
        .join(DATA_DIR);

    fs::create_dir_all(&data_dir).expect("Failed to create data directory");

    let gettext_dir = match env::var("FLATPAK_ID") {
        Ok(_) => GETTEXT_DIR_FLATPAK,
        // GETTEXT_DIR is where build.rs writes the catalogs. A system package installs them
        // elsewhere and sets OMATRIMIO_LOCALEDIR (e.g. /usr/share/locale) at build time.
        Err(_) => option_env!("OMATRIMIO_LOCALEDIR").unwrap_or(env!("GETTEXT_DIR")),
    };

    gettextrs::bindtextdomain(GETTEXT_DOMAIN, gettext_dir).expect("Failed to bind text domain");
    gettextrs::bind_textdomain_codeset(GETTEXT_DOMAIN, "UTF-8")
        .expect("Failed to set the text domain encoding");
    gettextrs::textdomain(GETTEXT_DOMAIN).expect("Failed to switch text domain");

    let library = unsafe { libloading::os::unix::Library::new("libepoxy.so.0") }
        .expect("Failed to load libepoxy");

    epoxy::load_with(|name| {
        unsafe { library.get::<_>(name.as_bytes()) }
            .map(|symbol| *symbol)
            .unwrap_or(ptr::null())
    });

    let args = Args::parse();

    // The web UI is loaded through the streaming server's proxy. Without the
    // server (no network on first run, no Node.js) the app still opens, on
    // the web UI directly, which says itself that streaming is unavailable.
    let server = Server::locate(&data_dir).and_then(|mut server| {
        server.start(args.dev)?;
        Ok(server)
    });
    let startup_url = match (&server, args.url.as_str()) {
        (Err(error), STARTUP_URL) => {
            error!("Starting without the streaming server: {error:#}");
            FALLBACK_URL.to_string()
        }
        _ => args.url,
    };

    let app = Application::new();
    app.set_property("dev-mode", args.dev);
    app.set_property("startup-url", startup_url);
    app.set_property("decorations", !args.no_window_decorations);

    let runtime = Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = runtime.enter();
    app.run(args.trailing)
}
