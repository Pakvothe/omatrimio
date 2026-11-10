use adw::prelude::AdwDialogExt;
use chrono::{Datelike, Utc};
use gtk::glib::object::IsA;

use crate::app::config::APP_NAME;

pub struct AboutDialog {
    dialog: adw::AboutDialog,
}

impl AboutDialog {
    pub fn new() -> Self {
        // omatrimio is a fork: the GPL asks that the original copyright stay
        // and that the changes be stated, and Stremio's trademark asks that
        // nobody mistake this for their app.
        let copyright = format!(
            "© {} omatrimio contributors\n© 2024-2026 Smart Code OOD (stremio-linux-shell)",
            Utc::now().year()
        );

        let dialog = adw::AboutDialog::builder()
            .application_icon("omatrimio")
            .application_name(APP_NAME)
            .version(env!("CARGO_PKG_VERSION"))
            .comments(
                "A Stremio-compatible media center with the Omarchy look.\n\n\
                 omatrimio is an unofficial, community-made client. It is not \
                 affiliated with, endorsed by, or supported by Stremio or Smart \
                 Code OOD. Stremio is a trademark of Smart Code OOD.\n\n\
                 It is a modified fork of stremio-linux-shell: it loads the \
                 Stremio web app and restyles it to follow the Omarchy theme.",
            )
            .website(env!("CARGO_PKG_HOMEPAGE"))
            .issue_url(concat!(env!("CARGO_PKG_REPOSITORY"), "/issues"))
            .license_type(gtk::License::Gpl30Only)
            .copyright(copyright)
            .developers(["omatrimio contributors", "Stremio (stremio-linux-shell)"])
            .build();
        dialog.add_link("stremio-linux-shell", "https://github.com/Stremio/stremio-linux-shell");

        Self { dialog }
    }

    pub fn show(&self, parent: &impl IsA<gtk::Widget>) {
        self.dialog.present(Some(parent));
    }
}
