import { useRef, useCallback } from 'react'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import type { SessionInfoYaml } from '@shared/SessionInfoYaml'

export interface LapRecord {
  lapNumber: number
  lapTimeSec: number
  airTemp: number
  trackTemp: number
  trackRubberState: string | null
  trackWetness: number | null
  isInLap: boolean
  isOutLap: boolean
  valid: boolean
  incidents: number
}

export interface LapHistoryResult {
  history: LapRecord[]
  reset: () => void
}

interface TrackerState {
  lastLapSeen: number
  lastSessionNum: number
  lastRecordedTime: number      // lapLastTime of the most recently recorded lap — guards against stale values
  onPitAtLapStart: boolean      // was on pit road when the current lap began
  wasOnPitDuringLap: boolean    // was on pit road at any point during the current lap
  incidentsAtLapStart: number | null
  history: LapRecord[]
}

export function useLapHistory(
  snap: TelemetrySnapshot | null,
  info: SessionInfoYaml | null,
): LapHistoryResult {
  const state = useRef<TrackerState>({
    lastLapSeen: -1,
    lastSessionNum: -1,
    lastRecordedTime: 0,
    onPitAtLapStart: false,
    wasOnPitDuringLap: false,
    incidentsAtLapStart: null,
    history: [],
  })

  const reset = useCallback(() => {
    const s = state.current
    s.history = []
    s.lastLapSeen = snap ? snap.lap : -1
    s.lastRecordedTime = 0
    s.onPitAtLapStart = snap ? snap.onPitRoad : false
    s.wasOnPitDuringLap = snap ? snap.onPitRoad : false
    s.incidentsAtLapStart = snap ? (snap.playerCarMyIncidentCount ?? 0) : null
  }, [snap])

  if (snap) {
    const s = state.current

    // Session change → full reset
    if (snap.sessionNum !== s.lastSessionNum) {
      s.lastLapSeen = snap.lap
      s.lastSessionNum = snap.sessionNum
      s.lastRecordedTime = 0
      s.onPitAtLapStart = snap.onPitRoad
      s.wasOnPitDuringLap = snap.onPitRoad
      s.incidentsAtLapStart = snap.playerCarMyIncidentCount ?? 0
      s.history = []
    }

    // Accumulate pit-road status during the current lap
    if (snap.onPitRoad) {
      s.wasOnPitDuringLap = true
    }

    // S/F crossing detected
    if (s.lastLapSeen >= 0 && snap.lap > s.lastLapSeen) {
      const lapTimeSec = snap.lapLastTime
      const incNow = snap.playerCarMyIncidentCount ?? 0

      // Guard against stale lapLastTime: iRacing sometimes updates LapLastLapTime
      // one or more frames after incrementing Lap. Only record once the value
      // differs from the previously recorded time (meaning iRacing has written
      // the fresh value). Until then, do NOT advance lastLapSeen so the crossing
      // is re-evaluated on the next frame.
      if (lapTimeSec > 0 && lapTimeSec !== s.lastRecordedTime) {
        const session = info?.SessionInfo?.Sessions?.find(
          (se) => se.SessionNum === snap.sessionNum
        )
        const rubber = session?.SessionTrackRubberState ?? null

        const incStart = s.incidentsAtLapStart
        const lapIncidents = incStart != null ? Math.max(0, incNow - incStart) : 0

        const isOutLap = s.onPitAtLapStart || s.wasOnPitDuringLap
        const isInLap = false
        const valid = !s.onPitAtLapStart && !s.wasOnPitDuringLap

        const record: LapRecord = {
          lapNumber: s.history.length + 1,
          lapTimeSec,
          airTemp: snap.airTemp,
          trackTemp: snap.trackTemp,
          trackRubberState: rubber,
          trackWetness: snap.trackWetness ?? null,
          isInLap,
          isOutLap,
          valid,
          incidents: lapIncidents,
        }
        s.history = [...s.history, record]

        s.lastRecordedTime = lapTimeSec
        s.incidentsAtLapStart = incNow
        // New lap starts at this S/F crossing — capture pit status as the "start" state
        s.onPitAtLapStart = snap.onPitRoad
        s.wasOnPitDuringLap = snap.onPitRoad
        s.lastLapSeen = snap.lap
      }
      // If lapTimeSec == 0 or unchanged: iRacing hasn't written the new time yet —
      // don't advance lastLapSeen so we re-check on the next frame.
    } else if (s.lastLapSeen < 0) {
      s.lastLapSeen = snap.lap
      s.onPitAtLapStart = snap.onPitRoad
      s.wasOnPitDuringLap = snap.onPitRoad
      s.incidentsAtLapStart = snap.playerCarMyIncidentCount ?? 0
    }
  }

  return { history: state.current.history, reset }
}
