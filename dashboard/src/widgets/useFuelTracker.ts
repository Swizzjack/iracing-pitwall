import { useCallback, useEffect, useRef, useState } from 'react'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'

export interface FuelTrackerResult {
  avgPerLap: number | null
  lastLapFuel: number | null
  worstLap: number | null
  lapCount: number
  reset: () => void
  // Per-lap snapshots — only update at S/F crossing or pit exit
  fuelAtLapStart: number | null
  lapLastTimeAtLapStart: number | null
  sessionTimeRemainAtLapStart: number | null
  sessionLapsRemainAtLapStart: number | null
  fuelUsePerHourAtLapStart: number | null
}

interface LapSnapshot {
  fuelLevel: number | null
  lapLastTime: number | null
  sessionTimeRemain: number | null
  sessionLapsRemain: number | null
  fuelUsePerHour: number | null
}

// Mutable per-frame bookkeeping. Lives in a ref and is only touched inside
// the effect below — render-visible results live in `Outputs` state.
interface TrackerState {
  lastLapSeen: number
  lastSessionNum: number
  fuelAtLapStart: number | null
  inPitLap: boolean
  prevOnPitRoad: boolean
}

interface Outputs {
  lapDeltas: number[]
  worstLap: number | null
  snapshot: LapSnapshot
}

function takeSnapshot(snap: TelemetrySnapshot): LapSnapshot {
  return {
    fuelLevel: snap.fuelLevel,
    lapLastTime: snap.lapLastTime > 0 ? snap.lapLastTime : null,
    sessionTimeRemain: snap.sessionTimeRemain ?? null,
    sessionLapsRemain: snap.sessionLapsRemain ?? null,
    fuelUsePerHour: snap.fuelUsePerHour > 0 ? snap.fuelUsePerHour : null,
  }
}

const NULL_SNAPSHOT: LapSnapshot = {
  fuelLevel: null,
  lapLastTime: null,
  sessionTimeRemain: null,
  sessionLapsRemain: null,
  fuelUsePerHour: null,
}

export function useFuelTracker(
  snap: TelemetrySnapshot | null,
  lapWindow: 3 | 5 | 'all',
): FuelTrackerResult {
  const state = useRef<TrackerState>({
    lastLapSeen: -1,
    lastSessionNum: -1,
    fuelAtLapStart: null,
    inPitLap: false,
    prevOnPitRoad: false,
  })
  const snapRef = useRef<TelemetrySnapshot | null>(null)
  const [outputs, setOutputs] = useState<Outputs>({
    lapDeltas: [],
    worstLap: null,
    snapshot: { ...NULL_SNAPSHOT },
  })

  // Frame processing happens in an effect (not during render): refs are
  // mutated freely here, and state is only committed on the rare events
  // that change what the widget shows (lap completed, session change,
  // pit exit) — the 60 Hz stream itself causes no extra renders.
  useEffect(() => {
    snapRef.current = snap
    if (!snap) return
    const s = state.current

    let sessionReset = false
    let completedDelta: number | null = null
    let snapshotNow = false

    // Reset on session change
    if (snap.sessionNum !== s.lastSessionNum) {
      s.lastLapSeen = snap.lap
      s.lastSessionNum = snap.sessionNum
      s.fuelAtLapStart = snap.fuelLevel
      s.inPitLap = snap.onPitRoad
      s.prevOnPitRoad = snap.onPitRoad
      sessionReset = true
      snapshotNow = true
    }

    // Track pit-road flag during current lap
    if (snap.onPitRoad) {
      s.inPitLap = true
    }

    // Detect lap change (S/F crossing)
    if (s.lastLapSeen >= 0 && snap.lap > s.lastLapSeen) {
      if (s.fuelAtLapStart !== null && !s.inPitLap) {
        const delta = s.fuelAtLapStart - snap.fuelLevel
        if (delta > 0) {
          completedDelta = delta
        }
      }
      s.fuelAtLapStart = snap.fuelLevel
      s.inPitLap = snap.onPitRoad
      s.lastLapSeen = snap.lap
      snapshotNow = true
    } else if (s.lastLapSeen < 0) {
      s.lastLapSeen = snap.lap
      s.fuelAtLapStart = snap.fuelLevel
      snapshotNow = true
    }

    // Pit exit: snapshot immediately with fresh (post-refuel) values
    const justExitedPit = s.prevOnPitRoad && !snap.onPitRoad
    if (justExitedPit) {
      s.fuelAtLapStart = snap.fuelLevel
      snapshotNow = true
    }
    s.prevOnPitRoad = snap.onPitRoad

    if (snapshotNow || completedDelta !== null) {
      const snapshot = takeSnapshot(snap)
      setOutputs((prev) => {
        const base = sessionReset ? [] : prev.lapDeltas
        const lapDeltas = completedDelta !== null ? [...base, completedDelta] : base
        const worstBase = sessionReset ? null : prev.worstLap
        const worstLap =
          completedDelta !== null && (worstBase === null || completedDelta > worstBase)
            ? completedDelta
            : worstBase
        return { lapDeltas, worstLap, snapshot }
      })
    }
  }, [snap])

  const reset = useCallback(() => {
    const s = state.current
    const sn = snapRef.current
    s.fuelAtLapStart = sn ? sn.fuelLevel : null
    s.inPitLap = false
    setOutputs({
      lapDeltas: [],
      worstLap: null,
      snapshot: sn ? takeSnapshot(sn) : { ...NULL_SNAPSHOT },
    })
  }, [])

  const deltas = outputs.lapDeltas
  const windowSlice = lapWindow === 'all' ? deltas : deltas.slice(-lapWindow)
  const avgPerLap = windowSlice.length > 0
    ? windowSlice.reduce((a, b) => a + b, 0) / windowSlice.length
    : null

  const lastLapFuel = deltas.length > 0 ? deltas[deltas.length - 1] : null

  return {
    avgPerLap,
    lastLapFuel,
    worstLap: outputs.worstLap,
    lapCount: deltas.length,
    reset,
    fuelAtLapStart: outputs.snapshot.fuelLevel,
    lapLastTimeAtLapStart: outputs.snapshot.lapLastTime,
    sessionTimeRemainAtLapStart: outputs.snapshot.sessionTimeRemain,
    sessionLapsRemainAtLapStart: outputs.snapshot.sessionLapsRemain,
    fuelUsePerHourAtLapStart: outputs.snapshot.fuelUsePerHour,
  }
}
