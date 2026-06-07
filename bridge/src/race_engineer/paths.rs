//! Application data paths for iRacing Pitwall.
//!
//! Windows: %APPDATA%\iRacingPitwall
//! Other:   ~/.local/share/iRacingPitwall

use std::path::PathBuf;

pub fn app_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(base).join("iRacingPitwall")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let base = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(base).join(".local").join("share").join("iRacingPitwall")
    }
}

pub fn piper_dir() -> PathBuf {
    app_data_dir().join("piper")
}

pub fn piper_exe() -> PathBuf {
    #[cfg(target_os = "windows")]
    { piper_dir().join("piper.exe") }
    #[cfg(not(target_os = "windows"))]
    { piper_dir().join("piper") }
}

pub fn voices_dir() -> PathBuf {
    app_data_dir().join("voices")
}

pub fn voice_model(voice_id: &str) -> PathBuf {
    voices_dir().join(format!("{voice_id}.onnx"))
}

pub fn voice_config(voice_id: &str) -> PathBuf {
    voices_dir().join(format!("{voice_id}.onnx.json"))
}
