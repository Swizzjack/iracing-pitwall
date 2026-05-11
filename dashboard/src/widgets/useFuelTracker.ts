import { useRef, useCallback } from 'react'
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

interface TrackerState {
  lastLapSeen: number
  lastSessionNum: number
  fuelAtLapStart: number | null
  lapDeltas: number[]
  worstLap: number | null
  inPitLap: boolean
  prevOnPitRoad: boolean
  snapshot: LapSnapshot
  resetToken: number
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
    lapDeltas: [],
    worstLap: null,
    inPitLap: false,
    prevOnPitRoad: false,
    snapshot: { ...NULL_SNAPSHOT },
    resetToken: 0,
  })

  const renderCount = useRef(0)

  const reset = useCallback(() => {
    const s = state.current
    s.lapDeltas = []
    s.worstLap = null
    s.fuelAtLapStart = snap ? snap.fuelLevel : null
    s.inPitLap = false
    s.snapshot = snap ? takeSnapshot(snap) : { ...NULL_SNAPSHOT }
    s.resetToken++
    renderCount.current++
  }, [snap])

  if (snap) {
    const s = state.current

    // Reset on session change
    if (snap.sessionNum !== s.lastSessionNum) {
      s.lastLapSeen = snap.lap
      s.lastSessionNum = snap.sessionNum
      s.fuelAtLapStart = snap.fuelLevel
      s.lapDeltas = []
      s.worstLap = null
      s.inPitLap = snap.onPitRoad
      s.prevOnPitRoad = snap.onPitRoad
      s.snapshot = takeSnapshot(snap)
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
          s.lapDeltas.push(delta)
          if (s.worstLap === null || delta > s.worstLap) s.worstLap = delta
        }
      }
      s.fuelAtLapStart = snap.fuelLevel
      s.inPitLap = snap.onPitRoad
      s.lastLapSeen = snap.lap
      s.snapshot = takeSnapshot(snap)
    } else if (s.lastLapSeen < 0) {
      s.lastLapSeen = snap.lap
      s.fuelAtLapStart = snap.fuelLevel
      s.snapshot = takeSnapshot(snap)
    }

    // Pit exit: snapshot immediately with fresh (post-refuel) values
    const justExitedPit = s.prevOnPitRoad && !snap.onPitRoad
    if (justExitedPit) {
      s.fuelAtLapStart = snap.fuelLevel
      s.snapshot = takeSnapshot(snap)
    }
    s.prevOnPitRoad = snap.onPitRoad
  }

  const s = state.current
  const deltas = s.lapDeltas

  const windowSlice = lapWindow === 'all' ? deltas : deltas.slice(-lapWindow)
  const avgPerLap = windowSlice.length > 0
    ? windowSlice.reduce((a, b) => a + b, 0) / windowSlice.length
    : null

  const lastLapFuel = deltas.length > 0 ? deltas[deltas.length - 1] : null

  return {
    avgPerLap,
    lastLapFuel,
    worstLap: s.worstLap,
    lapCount: deltas.length,
    reset,
    fuelAtLapStart: s.snapshot.fuelLevel,
    lapLastTimeAtLapStart: s.snapshot.lapLastTime,
    sessionTimeRemainAtLapStart: s.snapshot.sessionTimeRemain,
    sessionLapsRemainAtLapStart: s.snapshot.sessionLapsRemain,
    fuelUsePerHourAtLapStart: s.snapshot.fuelUsePerHour,
  }
}
