//! Race Engineer — voice commentary for iRacing sessions.
//!
//! Architecture:
//! - The blocking SDK loop sends `EngineerState` snapshots at ~10 Hz over an
//!   `mpsc::UnboundedSender<EngineerState>`.
//! - The async `run_engineer_task` consumes those snapshots, ticks the
//!   `RuleDispatcher`, synthesizes speech via a local Piper subprocess,
//!   and broadcasts `ServerMessage` (EngineerAudio etc.) to all WS clients.
//! - Frontend commands arrive as `ws::client::ClientMessage` variants and
//!   are routed here via a second `mpsc::UnboundedSender<ClientMessage>`.

pub mod audio;
pub mod config;
pub mod mod_types;
pub mod paths;
pub mod piper_binary;
pub mod rules;
pub mod state;
pub mod tts_engine;
pub mod voice_manager;

use tokio::sync::{broadcast, mpsc};

use crate::ws::client::ClientMessage;
use crate::ws::protocol::ServerMessage;
use rules::dispatcher::{EngineerBehavior, RuleDispatcher};
use rules::templates::TemplateRegistry;
use rules::FrequencyLevel;
use state::EngineerState;
use tts_engine::{SynthesisRequest, TtsEngine};
use voice_manager::{list_voices, VoiceInfo};

// ---------------------------------------------------------------------------
// Public channel type aliases (used in main.rs and handler.rs)
// ---------------------------------------------------------------------------

pub type EngineerStateTx = mpsc::UnboundedSender<EngineerState>;
pub type EngineerStateRx = mpsc::UnboundedReceiver<EngineerState>;
pub type EngineerCmdTx = mpsc::UnboundedSender<ClientMessage>;
pub type EngineerCmdRx = mpsc::UnboundedReceiver<ClientMessage>;
pub type EngineerAudioTx = broadcast::Sender<ServerMessage>;

// ---------------------------------------------------------------------------
// Main async engineer task
// ---------------------------------------------------------------------------

/// Spawn this as a Tokio task.  It runs for the lifetime of the process.
pub async fn run_engineer_task(
    mut state_rx: EngineerStateRx,
    mut cmd_rx: EngineerCmdRx,
    audio_tx: EngineerAudioTx,
) {
    let templates = TemplateRegistry::new();
    let behavior = EngineerBehavior::default();
    let rules = rules::build_default_rules();
    let mut dispatcher = RuleDispatcher::new(rules, behavior);
    let mut tts = TtsEngine::new();
    let mut previous_state: Option<EngineerState> = None;

    // Counters for synthesis request IDs
    let mut req_counter: u64 = 0;

    loop {
        tokio::select! {
            // --- incoming telemetry state ---
            maybe_state = state_rx.recv() => {
                let Some(current) = maybe_state else { break; };

                let events = dispatcher.tick(&current, previous_state.as_ref());
                previous_state = Some(current.clone());

                for event in events {
                    let voice_id = match &dispatcher.behavior.active_voice {
                        Some(v) if !v.is_empty() => v.clone(),
                        _ => continue,
                    };

                    // Inject pilot name if configured
                    let mut params = event.params;
                    if !dispatcher.behavior.mute_name {
                        if let Some(name) = &dispatcher.behavior.pilot_name {
                            if !name.is_empty() {
                                params = params.set("driver_name", name.clone());
                            }
                        }
                    }

                    let seed = dispatcher.next_render_seed();
                    let Some(text) = templates.render(event.template_key, &params, seed) else {
                        log::warn!("Engineer: no template for key '{}'", event.template_key);
                        continue;
                    };

                    req_counter += 1;
                    let request_id = format!("rule-{req_counter}");
                    let priority = event.priority.as_str().to_string();

                    match tts.synthesize(SynthesisRequest { text: text.clone(), voice_id }).await {
                        Ok(result) => {
                            let wav = audio::pcm_to_wav(&result.pcm, result.sample_rate);
                            let wav_base64 = audio::wav_to_base64(&wav);
                            let msg = ServerMessage::EngineerAudio {
                                request_id,
                                priority,
                                wav_base64,
                                sample_rate: result.sample_rate,
                                duration_ms: result.duration_ms,
                                text,
                            };
                            let _ = audio_tx.send(msg);
                        }
                        Err(e) => {
                            log::warn!("Engineer TTS error: {e}");
                        }
                    }
                }
            }

            // --- incoming commands from WS clients ---
            maybe_cmd = cmd_rx.recv() => {
                let Some(cmd) = maybe_cmd else { break; };
                handle_command(cmd, &mut dispatcher, &mut tts, &audio_tx).await;
            }
        }
    }

    log::info!("Engineer task exiting");
}

async fn handle_command(
    cmd: ClientMessage,
    dispatcher: &mut RuleDispatcher,
    tts: &mut TtsEngine,
    audio_tx: &EngineerAudioTx,
) {
    match cmd {
        ClientMessage::EngineerGetStatus => {
            let msg = build_status_msg();
            let _ = audio_tx.send(msg);
        }

        ClientMessage::EngineerInstallPiper => {
            let (prog_tx, mut prog_rx) = mpsc::unbounded_channel::<mod_types::DownloadProgress>();
            let audio_tx2 = audio_tx.clone();

            // Forward progress messages concurrently while install runs
            let progress_fwd = tokio::spawn(async move {
                while let Some(p) = prog_rx.recv().await {
                    let _ = audio_tx2.send(ServerMessage::EngineerInstallProgress {
                        target: p.target,
                        target_id: p.target_id,
                        bytes_downloaded: p.bytes_downloaded as u32,
                        bytes_total: p.bytes_total.map(|b| b as u32),
                        stage: p.stage,
                    });
                }
            });

            let install_result = tokio::task::spawn_blocking(move || piper_binary::install(prog_tx))
                .await
                .unwrap_or_else(|e| Err(anyhow::anyhow!("task panic: {e}")));

            // Drain any remaining progress messages before reporting completion
            let _ = progress_fwd.await;

            if let Err(ref e) = install_result {
                log::error!("Piper install failed: {e:#}");
            }

            let success = install_result.is_ok() && piper_binary::is_installed();
            let error_msg = install_result.err().map(|e| format!("{e:#}"));
            let _ = audio_tx.send(ServerMessage::EngineerInstallComplete {
                target: "piper".into(),
                target_id: None,
                success,
                error: if success { None } else { error_msg.or(Some("Installation failed".into())) },
            });
            // Refresh status
            let _ = audio_tx.send(build_status_msg());
        }

        ClientMessage::EngineerInstallVoice { voice_id } => {
            let (prog_tx, mut prog_rx) =
                mpsc::unbounded_channel::<mod_types::DownloadProgress>();
            let audio_tx2 = audio_tx.clone();
            let vid = voice_id.clone();

            let progress_fwd = tokio::spawn(async move {
                while let Some(p) = prog_rx.recv().await {
                    let _ = audio_tx2.send(ServerMessage::EngineerInstallProgress {
                        target: p.target,
                        target_id: p.target_id,
                        bytes_downloaded: p.bytes_downloaded as u32,
                        bytes_total: p.bytes_total.map(|b| b as u32),
                        stage: p.stage,
                    });
                }
            });

            let install_result = tokio::task::spawn_blocking(move || voice_manager::install_voice(&vid, prog_tx))
                .await
                .unwrap_or_else(|e| Err(anyhow::anyhow!("task panic: {e}")));

            let _ = progress_fwd.await;

            if let Err(ref e) = install_result {
                log::error!("Voice install failed ({voice_id}): {e:#}");
            }

            let success = install_result.is_ok() && voice_manager::is_installed(&voice_id);
            let error_msg = install_result.err().map(|e| format!("{e:#}"));
            let _ = audio_tx.send(ServerMessage::EngineerInstallComplete {
                target: "voice".into(),
                target_id: Some(voice_id),
                success,
                error: if success { None } else { error_msg.or(Some("Voice installation failed".into())) },
            });
            let _ = audio_tx.send(build_status_msg());
        }

        ClientMessage::EngineerUninstallVoice { voice_id } => {
            if let Err(e) = voice_manager::uninstall_voice(&voice_id) {
                log::warn!("Uninstall voice {voice_id}: {e}");
            }
            let _ = audio_tx.send(build_status_msg());
        }

        ClientMessage::EngineerSynthesize { voice_id, text, request_id } => {
            match tts
                .synthesize(SynthesisRequest { text: text.clone(), voice_id })
                .await
            {
                Ok(result) => {
                    let wav = audio::pcm_to_wav(&result.pcm, result.sample_rate);
                    let wav_base64 = audio::wav_to_base64(&wav);
                    let _ = audio_tx.send(ServerMessage::EngineerAudio {
                        request_id,
                        priority: "info".into(),
                        wav_base64,
                        sample_rate: result.sample_rate,
                        duration_ms: result.duration_ms,
                        text,
                    });
                }
                Err(e) => {
                    log::warn!("Engineer manual synthesize: {e}");
                    let _ = audio_tx.send(ServerMessage::EngineerError {
                        message: e.to_string(),
                    });
                }
            }
        }

        ClientMessage::EngineerUpdateBehavior {
            enabled,
            frequency,
            mute_in_qualifying,
            debug_all_rules_in_practice,
            active_voice_id,
            pilot_name,
            mute_name,
        } => {
            dispatcher.update_behavior(EngineerBehavior {
                enabled,
                frequency: FrequencyLevel::from_str(&frequency),
                mute_in_qualifying,
                debug_all_rules_in_practice,
                active_voice: active_voice_id,
                pilot_name,
                mute_name,
            });
            log::info!(
                "Engineer behavior updated: enabled={} frequency={frequency}",
                enabled
            );
        }

        // SDK-loop commands — should not arrive here, ignore
        ClientMessage::DeleteTrackMap { .. } | ClientMessage::SetSdkDebug { .. } => {}
    }
}

fn build_status_msg() -> ServerMessage {
    let piper_installed = piper_binary::is_installed();
    let piper_version = piper_binary::installed_version();
    let voices: Vec<VoiceInfo> = list_voices();
    ServerMessage::EngineerStatus {
        piper_installed,
        piper_version,
        voices,
    }
}
