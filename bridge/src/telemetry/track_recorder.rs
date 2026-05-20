use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::Result;
use crate::iracing_sdk::types::SessionInfoYaml;
use crate::iracing_sdk::IRacingClient;

// ─── Public types (exported to shared/) ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct TrackShape {
    pub track_key: String,
    pub track_name: String,
    pub track_config_name: String,
    /// 512 points resampled to uniform lapDistPct grid, loop-closed.
    pub points: Vec<TrackPoint>,
    /// World-space bounds: (xmin, ymin, xmax, ymax).
    pub bounds: [f32; 4],
    /// Elevation bounds: (zmin, zmax). Zero for tracks recorded before Z was added.
    #[serde(default)]
    pub z_bounds: [f32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct TrackPoint {
    /// Normalized lap distance [0, 1).
    pub p: f32,
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub z: f32,
}

// ─── Internal recording state ────────────────────────────────────────────────

const RESAMPLE_N: usize = 512;

struct RecordingState {
    samples: Vec<(f32, f64, f64, f64)>, // (p, x, y, z)
    integ_x: f64,
    integ_y: f64,
    integ_z: f64,
    last_p: f32,
    last_time: f64,
    initialized: bool,
}

impl RecordingState {
    fn new() -> Self {
        Self {
            samples: Vec::with_capacity(4096),
            integ_x: 0.0,
            integ_y: 0.0,
            integ_z: 0.0,
            last_p: -1.0,
            last_time: -1.0,
            initialized: false,
        }
    }

    fn push(&mut self, vx: f32, vy: f32, vz: f32, yaw: f32, session_time: f64, lap_dist_pct: f32) {
        if !self.initialized {
            self.last_time = session_time;
            self.last_p = lap_dist_pct;
            self.initialized = true;
            log::info!("track map: recording started at p={lap_dist_pct:.3}");
            return;
        }
        let n = self.samples.len();
        if n > 0 && n % 500 == 0 {
            log::debug!("track map: {n} samples accumulated, p={lap_dist_pct:.3}");
        }
        let dt = (session_time - self.last_time).clamp(0.0, 0.05);
        self.last_time = session_time;
        // Rotate body-frame velocity into world frame using yaw.
        let c = yaw.cos() as f64;
        let s = yaw.sin() as f64;
        let wvx = vx as f64 * c - vy as f64 * s;
        let wvy = vx as f64 * s + vy as f64 * c;
        self.integ_x += wvx * dt;
        self.integ_y += wvy * dt;
        // Z needs no yaw rotation; closed-loop drift is removed by the linear detrend in finalize().
        self.integ_z += vz as f64 * dt;
        self.samples
            .push((lap_dist_pct, self.integ_x, self.integ_y, self.integ_z));
        self.last_p = lap_dist_pct;
    }

    /// Build a TrackShape from accumulated samples: resample, close loop, compute bounds.
    fn finalize(
        mut self,
        track_key: &str,
        track_name: &str,
        track_config_name: &str,
    ) -> Option<TrackShape> {
        if self.samples.len() < 200 {
            return None;
        }
        // Sort by p just in case of minor reordering.
        self.samples.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // Linear detrend: remove drift between first and last sample so loop closes.
        let (p0, x0, y0, z0) = self.samples[0];
        let (p1, x1, y1, z1) = *self.samples.last().unwrap();
        let dp = (p1 - p0) as f64;
        let detrend = dp > 0.01;

        // Resample to RESAMPLE_N uniform p-steps.
        // Closed-loop drift correction: subtract a linear ramp proportional to (last - first)
        // so that the path starts and ends at the same world position.
        let mut points = Vec::with_capacity(RESAMPLE_N);
        for i in 0..RESAMPLE_N {
            let target_p = i as f32 / RESAMPLE_N as f32;
            let (sx, sy, sz) = interpolate(&self.samples, target_p);
            let (x, y, z) = if detrend {
                let t = ((target_p - p0) as f64 / dp).clamp(0.0, 1.0);
                (sx - (x1 - x0) * t, sy - (y1 - y0) * t, sz - (z1 - z0) * t)
            } else {
                (sx, sy, sz)
            };
            points.push(TrackPoint {
                p: target_p,
                x: x as f32,
                y: y as f32,
                z: z as f32,
            });
        }

        // Compute bounds.
        let mut xmin = f32::MAX;
        let mut ymin = f32::MAX;
        let mut zmin = f32::MAX;
        let mut xmax = f32::MIN;
        let mut ymax = f32::MIN;
        let mut zmax = f32::MIN;
        for pt in &points {
            if pt.x < xmin { xmin = pt.x; }
            if pt.x > xmax { xmax = pt.x; }
            if pt.y < ymin { ymin = pt.y; }
            if pt.y > ymax { ymax = pt.y; }
            if pt.z < zmin { zmin = pt.z; }
            if pt.z > zmax { zmax = pt.z; }
        }

        Some(TrackShape {
            track_key: track_key.to_string(),
            track_name: track_name.to_string(),
            track_config_name: track_config_name.to_string(),
            points,
            bounds: [xmin, ymin, xmax, ymax],
            z_bounds: [zmin, zmax],
        })
    }
}

fn interpolate(samples: &[(f32, f64, f64, f64)], target_p: f32) -> (f64, f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    // Binary search for the first sample with p >= target_p.
    let idx = samples.partition_point(|s| s.0 < target_p);
    if idx == 0 {
        return (samples[0].1, samples[0].2, samples[0].3);
    }
    if idx >= samples.len() {
        let last = samples.last().unwrap();
        return (last.1, last.2, last.3);
    }
    let (p0, x0, y0, z0) = samples[idx - 1];
    let (p1, x1, y1, z1) = samples[idx];
    let dp = (p1 - p0) as f64;
    if dp < 1e-10 {
        return (x0, y0, z0);
    }
    let t = ((target_p - p0) as f64 / dp).clamp(0.0, 1.0);
    (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, z0 + (z1 - z0) * t)
}

// ─── Public Recorder ─────────────────────────────────────────────────────────

pub struct TrackRecorder {
    cache_dir: PathBuf,
    current_key: Option<String>,
    pub shape: Option<TrackShape>,
    recording: Option<RecordingState>,
    /// Last known LapDistPct before the first S/F crossing; None = not yet on track.
    pre_arm_last_p: Option<f32>,
    /// Suppresses repeated "armed" log messages within a session.
    armed_logged: bool,
}

impl TrackRecorder {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            current_key: None,
            shape: None,
            recording: None,
            pre_arm_last_p: None,
            armed_logged: false,
        }
    }

    /// Called every 60-Hz frame. Manages track-change detection, recording, and finalization.
    pub fn update(&mut self, client: &IRacingClient, yaml: &SessionInfoYaml) -> Result<()> {
        let wi = &yaml.weekend_info;
        let config = wi.track_config_name.trim();
        let track_key = if config.is_empty() {
            wi.track_name.clone()
        } else {
            format!("{}_{}", wi.track_name, config.replace(' ', "_"))
        };

        // Track changed → reset and try to load from disk.
        if self.current_key.as_deref() != Some(&track_key) {
            self.current_key = Some(track_key.clone());
            self.shape = self.load_from_disk(&track_key);
            self.recording = None;
            self.pre_arm_last_p = None;
            self.armed_logged = false;
            if self.shape.is_some() {
                log::info!("track map: loaded shape for '{track_key}' from cache");
            } else {
                log::info!("track map: no cached shape for '{track_key}', will record");
            }
        }

        // If we already have a shape, nothing to do.
        if self.shape.is_some() {
            return Ok(());
        }

        // Skip recording when not driving.
        let is_on_track = client.get_bool("IsOnTrack").unwrap_or(false);
        let in_garage = client.get_bool("IsInGarage").unwrap_or(true);
        let on_pit = client.get_bool("OnPitRoad").unwrap_or(false);
        if !is_on_track || in_garage || on_pit {
            if self.recording.is_some() || self.pre_arm_last_p.is_some() {
                log::info!("track map: paused (off-track/garage/pit)");
            }
            self.recording = None;
            self.pre_arm_last_p = None;
            self.armed_logged = false;
            return Ok(());
        }

        let lap_dist_pct = client.get_f32("LapDistPct").unwrap_or(0.0);
        let session_time = client.get_f64("SessionTime").unwrap_or(0.0);
        let vx  = client.get_f32("VelocityX").unwrap_or(0.0);
        let vy  = client.get_f32("VelocityY").unwrap_or(0.0);
        let vz  = client.get_f32("VelocityZ").unwrap_or(0.0);
        let yaw = client.get_f32("Yaw").unwrap_or(0.0);

        // Pre-arm phase: wait for the first S/F crossing before accumulating samples.
        if self.recording.is_none() {
            let last = self.pre_arm_last_p;
            self.pre_arm_last_p = Some(lap_dist_pct);
            let crossed = matches!(last, Some(lp) if lp > 0.9 && lap_dist_pct < 0.1);
            if !crossed {
                if !self.armed_logged {
                    log::info!(
                        "track map: armed, waiting for S/F crossing (current p={lap_dist_pct:.3})"
                    );
                    self.armed_logged = true;
                }
                return Ok(());
            }
            log::info!("track map: S/F crossed, starting recording");
            self.recording = Some(RecordingState::new());
            self.pre_arm_last_p = None;
        }

        let rec = self.recording.as_mut().unwrap();

        // Detect backward teleport → restart. Exclude the natural S/F wrap
        // where last_p ≈ 0.99 flips to lap_dist_pct ≈ 0.005.
        let s_f_crossing = rec.last_p > 0.9 && lap_dist_pct < 0.1;
        if rec.last_p >= 0.0 && rec.last_p - lap_dist_pct > 0.1 && !s_f_crossing {
            let n = rec.samples.len();
            log::info!(
                "track map: teleport detected (last_p={:.3}, lap_dist_pct={:.3}, {n} samples discarded), restarting",
                rec.last_p, lap_dist_pct
            );
            self.recording = Some(RecordingState::new());
            return Ok(());
        }

        // Detect S/F crossing (p wraps from >0.95 back to <0.05).
        let lap_wrapped = rec.last_p > 0.95 && lap_dist_pct < 0.05 && rec.initialized;

        // Don't push the wrap-around sample: it has p≈0.01 but accumulates the full lap's
        // integ_x/y, which would corrupt the sort and make finalize()'s detrend fail.
        if !lap_wrapped {
            rec.push(vx, vy, vz, yaw, session_time, lap_dist_pct);
        }

        if lap_wrapped {
            let finished_rec = self.recording.take().unwrap();
            let n = finished_rec.samples.len();
            log::info!("track map: lap complete ({n} samples), finalizing…");
            match finished_rec.finalize(&track_key, &wi.track_name, &wi.track_config_name) {
                Some(shape) => {
                    log::info!("track map: shape built, saving to disk");
                    self.save_to_disk(&shape);
                    self.shape = Some(shape);
                }
                None => {
                    log::warn!("track map: finalization failed (too few samples), retrying");
                    self.recording = Some(RecordingState::new());
                }
            }
        }

        Ok(())
    }

    fn cache_path(&self, track_key: &str) -> PathBuf {
        let safe_key: String = track_key
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        self.cache_dir.join(format!("{safe_key}.json"))
    }

    fn load_from_disk(&self, track_key: &str) -> Option<TrackShape> {
        let path = self.cache_path(track_key);
        let data = std::fs::read(&path).ok()?;
        serde_json::from_slice(&data)
            .map_err(|e| log::warn!("track map: failed to parse cache {}: {e}", path.display()))
            .ok()
    }

    fn save_to_disk(&self, shape: &TrackShape) {
        if let Err(e) = std::fs::create_dir_all(&self.cache_dir) {
            log::warn!("track map: cannot create cache dir: {e}");
            return;
        }
        let path = self.cache_path(&shape.track_key);
        match serde_json::to_vec_pretty(shape) {
            Ok(data) => {
                if let Err(e) = std::fs::write(&path, data) {
                    log::warn!("track map: failed to write cache {}: {e}", path.display());
                } else {
                    log::info!("track map: saved to {}", path.display());
                }
            }
            Err(e) => log::warn!("track map: serialization failed: {e}"),
        }
    }
}

/// Helper so callers can get a reference to the track path.
pub fn cache_dir_from_exe() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("cache").join("tracks")))
        .unwrap_or_else(|| Path::new("cache/tracks").to_path_buf())
}
