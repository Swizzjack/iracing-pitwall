//! Piper version pin and bundled voice catalogue.

pub const PIPER_VERSION: &str = "2023.11.14-2";

/// Windows release download URL for Piper TTS.
pub const PIPER_DOWNLOAD_URL: &str =
    "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_windows_amd64.zip";

/// A voice available for download and use with Piper.
#[derive(Debug, Clone)]
pub struct VoiceDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub language: &'static str,
    pub description: &'static str,
    pub model_url: &'static str,
    pub config_url: &'static str,
    pub sample_url: &'static str,
    pub size_mb: u32,
}

/// All voices available in this build.
pub static AVAILABLE_VOICES: &[VoiceDefinition] = &[
    VoiceDefinition {
        id: "cori-gb-high",
        name: "Cori",
        language: "en-GB",
        description: "British female, clear and calm",
        model_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_GB/cori/high/en_GB-cori-high.onnx",
        config_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_GB/cori/high/en_GB-cori-high.onnx.json",
        sample_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_GB/cori/high/samples/speaker_0.mp3",
        size_mb: 110,
    },
    VoiceDefinition {
        id: "danny-us-low",
        name: "Danny",
        language: "en-US",
        description: "US male, deep and grounded",
        model_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/danny/low/en_US-danny-low.onnx",
        config_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/danny/low/en_US-danny-low.onnx.json",
        sample_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/danny/low/samples/speaker_0.mp3",
        size_mb: 25,
    },
    VoiceDefinition {
        id: "northern-english-male-medium",
        name: "Northern English",
        language: "en-GB",
        description: "British male, classic race engineer vibe",
        model_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_GB/northern_english_male/medium/en_GB-northern_english_male-medium.onnx",
        config_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_GB/northern_english_male/medium/en_GB-northern_english_male-medium.onnx.json",
        sample_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_GB/northern_english_male/medium/samples/speaker_0.mp3",
        size_mb: 60,
    },
    VoiceDefinition {
        id: "joe-us-medium",
        name: "Joe",
        language: "en-US",
        description: "US male, neutral and professional",
        model_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/joe/medium/en_US-joe-medium.onnx",
        config_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/joe/medium/en_US-joe-medium.onnx.json",
        sample_url: "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/joe/medium/samples/speaker_0.mp3",
        size_mb: 60,
    },
];
