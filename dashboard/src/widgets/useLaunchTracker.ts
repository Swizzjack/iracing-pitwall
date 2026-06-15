import { useCallback, useEffect, useRef, useState } from 'react'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'

// SDK LatAccel/LongAccel/VertAccel are raw m/s² (see TelemetryInputs.tsx MAX_G comment).
const G = 9.80665

const STOPPED_MS = 0.3                       // ~1 km/h — considered stationary
const LAUNCH_MS = 0.6                        // ~2 km/h — movement that starts a recording
const TARGET_100KMH_MS = 100 / 3.6           // 27.778 m/s
const TARGET_60MPH_MS = (60 * 1.609344) / 3.6 // 26.822 m/s — always crossed before 100 km/h
const PREROLL_SEC = 1.5                      // pre-launch context kept while armed
const MAX_RECORD_SEC = 30                    // abort a recording that never reaches 100 km/h
const ABORT_DROP_MS = 1.0                    // speed drop from peak that aborts a recording

export type LaunchPhase = 'idle' | 'armed' | 'recording'

export interface LaunchSample {
  t: number           // seconds, 0 = launch start, negative = pre-roll
  speedMs: number
  longAccelG: number
}

export interface LaunchResult {
  samples: LaunchSample[]
  time100kmh: number
  time60mph: number
  peakAccelG: number
  peakSpeedMs: number
}

export interface LaunchTrackerResult {
  phase: LaunchPhase
  liveSpeedMs: number
  liveElapsed: number
  lastResult: LaunchResult | null
  clearResult: () => void
}

// Mutable per-frame bookkeeping; only touched inside the effect below.
// The render-visible phase/result live in useState.
interface TrackerState {
  phase: LaunchPhase
  preroll: LaunchSample[]
  recording: LaunchSample[]
  recordStart: number
  peakSpeed: number
  lastResult: LaunchResult | null
}

// Linear interpolation of the time at which `speedMs` first reaches `target`.
function interpolateCrossing(samples: LaunchSample[], target: number): number {
  for (let i = 1; i < samples.length; i++) {
    const a = samples[i - 1]
    const b = samples[i]
    if (a.speedMs < target && b.speedMs >= target) {
      const frac = (target - a.speedMs) / (b.speedMs - a.speedMs)
      return a.t + frac * (b.t - a.t)
    }
  }
  return samples[samples.length - 1]?.t ?? 0
}

export function useLaunchTracker(snap: TelemetrySnapshot | null): LaunchTrackerResult {
  const state = useRef<TrackerState>({
    phase: 'idle',
    preroll: [],
    recording: [],
    recordStart: 0,
    peakSpeed: 0,
    lastResult: null,
  })

  const [phase, setPhase] = useState<LaunchPhase>('idle')
  const [liveSpeedMs, setLiveSpeedMs] = useState(0)
  const [liveElapsed, setLiveElapsed] = useState(0)
  const [lastResult, setLastResult] = useState<LaunchResult | null>(null)

  useEffect(() => {
    if (!snap) return
    const s = state.current
    const prevPhase = s.phase
    const prevResult = s.lastResult

    const t = performance.now() / 1000
    const speedMs = snap.speedMs
    const longAccelG = snap.longAccel / G
    const onTrack = snap.isOnTrackCar && !snap.isInGarage

    if (!onTrack) {
      if (s.phase !== 'idle') {
        s.phase = 'idle'
        s.preroll = []
        s.recording = []
      }
    } else if (s.phase === 'idle') {
      if (speedMs < STOPPED_MS) {
        s.phase = 'armed'
        s.preroll = [{ t, speedMs, longAccelG }]
      }
    } else if (s.phase === 'armed') {
      if (speedMs >= LAUNCH_MS) {
        // Launch detected — t=0 is now; shift pre-roll samples to negative times.
        s.recordStart = t
        s.peakSpeed = speedMs
        s.recording = [
          ...s.preroll.map(p => ({ ...p, t: p.t - t })),
          { t: 0, speedMs, longAccelG },
        ]
        s.preroll = []
        s.phase = 'recording'
      } else {
        s.preroll.push({ t, speedMs, longAccelG })
        const cutoff = t - PREROLL_SEC
        while (s.preroll.length > 0 && s.preroll[0].t < cutoff) s.preroll.shift()
      }
    } else {
      // recording
      const relT = t - s.recordStart
      s.recording.push({ t: relT, speedMs, longAccelG })
      if (speedMs > s.peakSpeed) s.peakSpeed = speedMs

      if (speedMs >= TARGET_100KMH_MS) {
        const samples = s.recording
        s.lastResult = {
          samples,
          time100kmh: interpolateCrossing(samples, TARGET_100KMH_MS),
          time60mph: interpolateCrossing(samples, TARGET_60MPH_MS),
          peakAccelG: Math.max(...samples.map(p => p.longAccelG)),
          peakSpeedMs: s.peakSpeed,
        }
        s.phase = 'idle'
        s.recording = []
      } else if (relT > MAX_RECORD_SEC || speedMs < s.peakSpeed - ABORT_DROP_MS) {
        // Timed out or aborted (spin/crash/lift) before reaching 100 km/h — discard.
        s.phase = 'idle'
        s.recording = []
      }
    }

    if (s.phase !== prevPhase) setPhase(s.phase)
    if (s.lastResult !== prevResult) setLastResult(s.lastResult)
    if (s.phase === 'recording') {
      setLiveSpeedMs(speedMs)
      setLiveElapsed(t - s.recordStart)
    }
  }, [snap])

  const clearResult = useCallback(() => {
    state.current.lastResult = null
    setLastResult(null)
  }, [])

  return { phase, liveSpeedMs, liveElapsed, lastResult, clearResult }
}
