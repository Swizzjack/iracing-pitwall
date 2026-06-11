import { useCallback, useEffect, useRef, useState } from 'react'
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

// Mutable per-frame bookkeeping; only touched inside the effect below.
// The render-visible lap list lives in useState.
interface TrackerState {
  lastLapSeen: number
  lastSessionNum: number
  lastSubSessionId: bigint | null
  lastRecordedTime: number      // lapLastTime of the most recently recorded lap — guards against stale values
  onPitAtLapStart: boolean      // was on pit road when the current lap began
  wasOnPitDuringLap: boolean    // was on pit road at any point during the current lap
  incidentsAtLapStart: number | null
}

export function useLapHistory(
  snap: TelemetrySnapshot | null,
  info: SessionInfoYaml | null,
): LapHistoryResult {
  const state = useRef<TrackerState>({
    lastLapSeen: -1,
    lastSessionNum: -1,
    lastSubSessionId: null,
    lastRecordedTime: 0,
    onPitAtLapStart: false,
    wasOnPitDuringLap: false,
    incidentsAtLapStart: null,
  })
  const snapRef = useRef<TelemetrySnapshot | null>(null)
  const [history, setHistory] = useState<LapRecord[]>([])

  useEffect(() => {
    snapRef.current = snap
    if (!snap) return
    const s = state.current

    // Session change or new server (SubSessionID changes) → full reset
    const subId = info?.WeekendInfo?.SubSessionID ?? null
    const sessionChanged = snap.sessionNum !== s.lastSessionNum
    const serverChanged = s.lastSubSessionId !== null && subId !== null && subId !== s.lastSubSessionId
    if (sessionChanged || serverChanged) {
      s.lastLapSeen = snap.lap
      s.lastSessionNum = snap.sessionNum
      s.lastSubSessionId = subId
      s.lastRecordedTime = snap.lapLastTime
      s.onPitAtLapStart = snap.onPitRoad
      s.wasOnPitDuringLap = snap.onPitRoad
      s.incidentsAtLapStart = snap.playerCarMyIncidentCount ?? 0
      setHistory([])
      return
    }
    if (s.lastSubSessionId === null && subId !== null) {
      s.lastSubSessionId = subId
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
        const valid = !s.onPitAtLapStart && !s.wasOnPitDuringLap

        const record: Omit<LapRecord, 'lapNumber'> = {
          lapTimeSec,
          airTemp: snap.airTemp,
          trackTemp: snap.trackTemp,
          trackRubberState: rubber,
          trackWetness: snap.trackWetness ?? null,
          isInLap: false,
          isOutLap,
          valid,
          incidents: lapIncidents,
        }
        setHistory((prev) => [...prev, { ...record, lapNumber: prev.length + 1 }])

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
  }, [snap, info])

  const reset = useCallback(() => {
    const s = state.current
    const sn = snapRef.current
    s.lastLapSeen = sn ? sn.lap : -1
    s.lastRecordedTime = sn ? sn.lapLastTime : 0
    s.onPitAtLapStart = sn ? sn.onPitRoad : false
    s.wasOnPitDuringLap = sn ? sn.onPitRoad : false
    s.incidentsAtLapStart = sn ? (sn.playerCarMyIncidentCount ?? 0) : null
    setHistory([])
  }, [])

  return { history, reset }
}
