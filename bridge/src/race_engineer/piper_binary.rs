//! Piper TTS binary — download and installation.

use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use tokio::sync::mpsc;

use super::config::{PIPER_DOWNLOAD_URL, PIPER_VERSION};
use super::mod_types::DownloadProgress;
use super::paths::{piper_dir, piper_exe};

fn build_agent() -> Result<ureq::Agent> {
    let tls = native_tls::TlsConnector::new().context("init TLS connector")?;
    Ok(ureq::AgentBuilder::new()
        .tls_connector(Arc::new(tls))
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(120))
        .build())
}

/// Returns true if the Piper executable is present.
pub fn is_installed() -> bool {
    piper_exe().exists()
}

/// Returns the installed version string from a sentinel file, or None.
pub fn installed_version() -> Option<String> {
    let ver_file = piper_dir().join("VERSION");
    std::fs::read_to_string(ver_file).ok()
}

/// Download and install Piper.  Runs blocking (call via `spawn_blocking`).
pub fn install(progress_tx: mpsc::UnboundedSender<DownloadProgress>) -> Result<()> {
    let dir = piper_dir();
    std::fs::create_dir_all(&dir).context("create piper dir")?;

    let url = PIPER_DOWNLOAD_URL;
    let target = "piper";

    // --- Download ---
    let zip_bytes = download_file(url, target, None, &progress_tx)?;

    // --- Extract ---
    let _ = progress_tx.send(DownloadProgress {
        bytes_downloaded: zip_bytes.len() as u64,
        bytes_total: Some(zip_bytes.len() as u64),
        stage: "extracting".into(),
        target: target.into(),
        target_id: None,
    });

    extract_zip(&zip_bytes, &dir).context("extract piper zip")?;

    // --- Validate ---
    let _ = progress_tx.send(DownloadProgress {
        bytes_downloaded: 0,
        bytes_total: None,
        stage: "validating".into(),
        target: target.into(),
        target_id: None,
    });

    if !piper_exe().exists() {
        return Err(anyhow!("piper executable not found after extraction"));
    }

    // Write version sentinel
    let ver_path = dir.join("VERSION");
    if let Ok(mut f) = std::fs::File::create(ver_path) {
        let _ = f.write_all(PIPER_VERSION.as_bytes());
    }

    log::info!("Piper installed at {}", piper_exe().display());
    Ok(())
}

fn extract_zip(zip_bytes: &[u8], dest: &Path) -> Result<()> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor).context("open zip")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        // enclosed_name() rejects entries that would escape the target dir
        // (absolute paths, `..` components) — zip-slip protection.
        let Some(safe_path) = file.enclosed_name() else {
            return Err(anyhow!("zip entry escapes target dir: {}", file.name()));
        };

        // Strip leading "piper/" prefix if present
        let rel = safe_path
            .strip_prefix("piper")
            .map(|p| p.to_path_buf())
            .unwrap_or(safe_path);

        if rel.as_os_str().is_empty() || file.is_dir() {
            continue;
        }

        let out_path = dest.join(rel);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut out = std::fs::File::create(&out_path)
            .with_context(|| format!("create {}", out_path.display()))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        out.write_all(&buf)?;
    }
    Ok(())
}

/// Blocking HTTP GET with progress reporting.
pub fn download_file(
    url: &str,
    target: &str,
    target_id: Option<&str>,
    progress_tx: &mpsc::UnboundedSender<DownloadProgress>,
) -> Result<Vec<u8>> {
    const CHUNK: usize = 65_536;

    let agent = build_agent()?;
    let response = agent
        .get(url)
        .call()
        .with_context(|| format!("GET {url}"))?;

    let content_length: Option<u64> = response
        .header("content-length")
        .and_then(|v| v.parse().ok());

    let mut reader = response.into_reader();
    let mut data = Vec::new();
    let mut buf = vec![0u8; CHUNK];

    // Throttle progress messages: every chunk (64 KiB) would mean thousands of
    // WS broadcasts per voice download. One update per interval is plenty for a
    // progress bar; a final message after the loop reports the exact total.
    const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);
    let mut last_progress: Option<std::time::Instant> = None;

    loop {
        use std::io::Read;
        let n = reader.read(&mut buf).context("read response body")?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buf[..n]);
        if last_progress.map_or(true, |t| t.elapsed() >= PROGRESS_INTERVAL) {
            last_progress = Some(std::time::Instant::now());
            let _ = progress_tx.send(DownloadProgress {
                bytes_downloaded: data.len() as u64,
                bytes_total: content_length,
                stage: "downloading".into(),
                target: target.into(),
                target_id: target_id.map(|s| s.to_string()),
            });
        }
    }
    let _ = progress_tx.send(DownloadProgress {
        bytes_downloaded: data.len() as u64,
        bytes_total: content_length,
        stage: "downloading".into(),
        target: target.into(),
        target_id: target_id.map(|s| s.to_string()),
    });

    if data.is_empty() {
        return Err(anyhow!("empty response from {url}"));
    }

    Ok(data)
}
