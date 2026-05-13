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
}

export interface LapHistoryResult {
  history: LapRecord[]
  reset: () => void
}

interface TrackerState {
  lastLapSeen: number
  lastSessionNum: number
  inPitLap: boolean
  prevOnPitRoad: boolean
  wasInPitOnPrevLap: boolean
  history: LapRecord[]
}

export function useLapHistory(
  snap: TelemetrySnapshot | null,
  info: SessionInfoYaml | null,
): LapHistoryResult {
  const state = useRef<TrackerState>({
    lastLapSeen: -1,
    lastSessionNum: -1,
    inPitLap: false,
    prevOnPitRoad: false,
    wasInPitOnPrevLap: false,
    history: [],
  })

  const reset = useCallback(() => {
    const s = state.current
    s.history = []
    s.lastLapSeen = snap ? snap.lap : -1
    s.inPitLap = snap ? snap.onPitRoad : false
    s.prevOnPitRoad = snap ? snap.onPitRoad : false
    s.wasInPitOnPrevLap = false
  }, [snap])

  if (snap) {
    const s = state.current

    // Session change → full reset
    if (snap.sessionNum !== s.lastSessionNum) {
      s.lastLapSeen = snap.lap
      s.lastSessionNum = snap.sessionNum
      s.inPitLap = snap.onPitRoad
      s.prevOnPitRoad = snap.onPitRoad
      s.wasInPitOnPrevLap = false
      s.history = []
    }

    if (snap.onPitRoad) {
      s.inPitLap = true
    }

    // S/F crossing detected
    if (s.lastLapSeen >= 0 && snap.lap > s.lastLapSeen) {
      const lapTimeSec = snap.lapLastTime
      const wasInPit = s.inPitLap

      if (lapTimeSec > 0) {
        const session = info?.SessionInfo?.Sessions?.find(
          (se) => se.SessionNum === snap.sessionNum
        )
        const rubber = session?.SessionTrackRubberState ?? null

        const record: LapRecord = {
          lapNumber: s.lastLapSeen,
          lapTimeSec,
          airTemp: snap.airTemp,
          trackTemp: snap.trackTemp,
          trackRubberState: rubber,
          trackWetness: snap.trackWetness ?? null,
          isInLap: wasInPit,
          isOutLap: s.wasInPitOnPrevLap && !wasInPit,
          valid: !wasInPit && !s.wasInPitOnPrevLap,
        }
        s.history = [...s.history, record]
      }

      s.wasInPitOnPrevLap = wasInPit
      s.inPitLap = snap.onPitRoad
      s.lastLapSeen = snap.lap
    } else if (s.lastLapSeen < 0) {
      s.lastLapSeen = snap.lap
      s.inPitLap = snap.onPitRoad
    }

    s.prevOnPitRoad = snap.onPitRoad
  }

  return { history: state.current.history, reset }
}
