use std::{
    env, fs,
    io::{BufRead, BufReader},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{self, Child, Command},
    thread,
};

use anyhow::{Context, bail};
use tracing::{debug, info};

use crate::config::IPC_KEY;

/// Stremio's streaming server, which serves the web UI's proxy, torrents, and
/// transcoding. It is Stremio's own closed-source program, so omatrimio does
/// not ship it: it is fetched from Stremio's download site on first run,
/// pinned to this release and checksum, and kept in the data directory.
const SERVER_VERSION: &str = "v4.22.4";
const SERVER_SHA256: &str = "0b5b0bfc0165dad7f6397335594050832aa5760d7098123f739292efb80794ba";

/// A package may install the server itself; it is used when present.
const SYSTEM_SERVER: &str = "/usr/lib/omatrimio/server.js";

pub struct Server {
    process: Option<Child>,
    file: PathBuf,
}

impl Server {
    /// Finds the server: `SERVER_PATH` if set, then a packaged copy, then the
    /// copy in the data directory, downloading it there if it is missing.
    pub fn locate(data_dir: &Path) -> anyhow::Result<Self> {
        let file = match env::var_os("SERVER_PATH") {
            Some(path) => PathBuf::from(path),
            None if Path::new(SYSTEM_SERVER).exists() => PathBuf::from(SYSTEM_SERVER),
            None => {
                let file = data_dir.join(format!("server-{SERVER_VERSION}.js"));
                if !file.exists() {
                    download(&file)?;
                }
                file
            }
        };

        Ok(Self {
            process: None,
            file,
        })
    }

    pub fn start(&mut self, dev: bool) -> anyhow::Result<()> {
        let mut command = Command::new("node");
        command
            .env("NO_CORS", (dev as i32).to_string())
            .env("SERVER_IPC_KEY", IPC_KEY)
            .arg(self.file.as_os_str())
            .stdout(process::Stdio::piped())
            .process_group(0);

        unsafe {
            command.pre_exec(move || {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }

        let mut child = command
            .spawn()
            .context("Failed to run node; the streaming server needs Node.js")?;

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            thread::spawn(move || {
                while let Some(Ok(line)) = lines.next() {
                    debug!(target: "server", "{}", line);
                }
            });
        }

        self.process = Some(child);

        Ok(())
    }

    pub fn stop(&mut self) -> anyhow::Result<()> {
        if let Some(mut process) = self.process.take() {
            process.kill().context("Failed to kill server process")?;
        }

        Ok(())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop().expect("Failed to stop server");
    }
}

/// Downloads the pinned server beside its final path, checks it, and renames
/// it into place, so a partial or tampered download is never run.
fn download(file: &Path) -> anyhow::Result<()> {
    let url = format!("https://dl.strem.io/server/{SERVER_VERSION}/desktop/server.js");
    let partial = file.with_extension("js.part");
    info!("Downloading Stremio's streaming server {SERVER_VERSION} from {url}");

    let status = Command::new("curl")
        .args(["--fail", "--silent", "--show-error", "--location", "--max-time", "120"])
        .arg("--output")
        .arg(&partial)
        .arg(&url)
        .status()
        .context("Failed to run curl")?;
    if !status.success() {
        let _ = fs::remove_file(&partial);
        bail!("Could not download the streaming server from {url}");
    }

    let checksum = sha256(&partial)?;
    if checksum != SERVER_SHA256 {
        let _ = fs::remove_file(&partial);
        bail!("The downloaded streaming server doesn't match its checksum (got {checksum})");
    }

    fs::rename(&partial, file).context("Failed to put the streaming server in place")?;
    Ok(())
}

fn sha256(file: &Path) -> anyhow::Result<String> {
    let output = Command::new("sha256sum")
        .arg(file)
        .output()
        .context("Failed to run sha256sum")?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.split_whitespace()
        .next()
        .map(str::to_string)
        .context("sha256sum printed nothing")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_reads_a_file() {
        let file = env::temp_dir().join(format!("omatrimio-sha-{}", process::id()));
        fs::write(&file, "omatrimio\n").unwrap();
        let checksum = sha256(&file).unwrap();
        fs::remove_file(&file).unwrap();
        assert_eq!(checksum.len(), 64);
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
