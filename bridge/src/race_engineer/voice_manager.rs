//! Voice model download and installation management.

use std::io::Write;

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use tokio::sync::mpsc;
use ts_rs::TS;

use super::config::{VoiceDefinition, AVAILABLE_VOICES};
use super::mod_types::DownloadProgress;
use super::paths::{voice_config, voice_model, voices_dir};
use super::piper_binary::download_file;

/// Voice status info sent to the frontend.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub language: String,
    pub description: String,
    pub size_mb: u32,
    pub installed: bool,
    pub sample_url: String,
}

/// Returns status for all known voices.
pub fn list_voices() -> Vec<VoiceInfo> {
    AVAILABLE_VOICES
        .iter()
        .map(|v| VoiceInfo {
            id: v.id.to_string(),
            name: v.name.to_string(),
            language: v.language.to_string(),
            description: v.description.to_string(),
            size_mb: v.size_mb,
            installed: is_installed(v.id),
            sample_url: v.sample_url.to_string(),
        })
        .collect()
}

/// Returns true if both the model and config files exist for the given voice ID.
pub fn is_installed(voice_id: &str) -> bool {
    voice_model(voice_id).exists() && voice_config(voice_id).exists()
}

/// Download and install a voice.  Runs blocking (call via `spawn_blocking`).
pub fn install_voice(
    voice_id: &str,
    progress_tx: mpsc::UnboundedSender<DownloadProgress>,
) -> Result<()> {
    let def = find_voice(voice_id)?;
    let voices_dir = voices_dir();
    std::fs::create_dir_all(&voices_dir).context("create voices dir")?;

    let model_path = voice_model(def.id);
    let config_path = voice_config(def.id);
    let tmp_model = voices_dir.join(format!("{}.onnx.tmp", def.id));
    let tmp_config = voices_dir.join(format!("{}.onnx.json.tmp", def.id));

    // Download model
    let model_bytes = download_file(def.model_url, "voice", Some(def.id), &progress_tx)
        .with_context(|| format!("download model for {voice_id}"))?;

    if model_bytes.len() < 10 * 1024 * 1024 {
        return Err(anyhow!("voice model too small — download may have failed"));
    }

    let mut f = std::fs::File::create(&tmp_model).context("create tmp model")?;
    f.write_all(&model_bytes).context("write tmp model")?;
    drop(f);

    // Download config
    let config_bytes = download_file(def.config_url, "voice", Some(def.id), &progress_tx)
        .with_context(|| format!("download config for {voice_id}"))?;

    if config_bytes.len() > 1024 * 1024 {
        return Err(anyhow!("voice config suspiciously large"));
    }

    let mut f = std::fs::File::create(&tmp_config).context("create tmp config")?;
    f.write_all(&config_bytes).context("write tmp config")?;
    drop(f);

    // Validate stage
    let _ = progress_tx.send(DownloadProgress {
        bytes_downloaded: 0,
        bytes_total: None,
        stage: "validating".into(),
        target: "voice".into(),
        target_id: Some(def.id.to_string()),
    });

    // Atomic rename
    std::fs::rename(&tmp_model, &model_path).context("rename model")?;
    std::fs::rename(&tmp_config, &config_path).context("rename config")?;

    log::info!("Voice '{}' installed at {}", def.id, model_path.display());
    Ok(())
}

/// Remove installed voice files.
pub fn uninstall_voice(voice_id: &str) -> Result<()> {
    let model = voice_model(voice_id);
    let config = voice_config(voice_id);
    if model.exists() {
        std::fs::remove_file(&model).context("remove model")?;
    }
    if config.exists() {
        std::fs::remove_file(&config).context("remove config")?;
    }
    log::info!("Voice '{}' uninstalled", voice_id);
    Ok(())
}

fn find_voice(id: &str) -> Result<&'static VoiceDefinition> {
    AVAILABLE_VOICES
        .iter()
        .find(|v| v.id == id)
        .ok_or_else(|| anyhow!("Unknown voice id: {id}"))
}
