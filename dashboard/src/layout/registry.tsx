import { memo, type ReactNode } from 'react'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import type { StandingsSnapshot } from '@shared/StandingsSnapshot'
import type { SessionInfoYaml } from '@shared/SessionInfoYaml'
import type { TrackMapSnapshot } from '@shared/TrackMapSnapshot'
import { Standings } from '../widgets/Standings'
import { TelemetryInputs } from '../widgets/TelemetryInputs'
import { TrackMap } from '../widgets/TrackMap'
import { SoF } from '../widgets/SoF'
import { Weather } from '../widgets/Weather'
import { Fuel } from '../widgets/Fuel'
import { Electronics } from '../widgets/Electronics'
import { Tire } from '../widgets/Tire'
import { LapHistory } from '../widgets/LapHistory'
import { Wind } from '../widgets/Wind'
import { EngineerTranscript } from '../widgets/EngineerTranscript'

export type WidgetData = {
  tel: TelemetrySnapshot | null
  standings: StandingsSnapshot | null
  info: SessionInfoYaml | null
  trackMap: TrackMapSnapshot | null
  onDeleteTrackMap: (trackKey: string) => void
}

export type WidgetDef = {
  id: string
  title: string
  render: (data: WidgetData) => ReactNode
  default: { w: number; h: number; minW: number; minH: number }
}

// Every widget is memoized at this single render boundary: App re-renders at
// telemetry rate (rAF-throttled ~60 Hz), but most widgets consume snapshots
// that only change at 4–15 Hz. With the stable prop identities provided by
// App (useMemo'd WidgetData, useCallback'd onDeleteTrackMap), memo() skips
// the re-render whenever a widget's own inputs are unchanged.
const TelemetryInputsM = memo(TelemetryInputs)
const StandingsM = memo(Standings)
const SoFM = memo(SoF)
const TrackMapM = memo(TrackMap)
const WeatherM = memo(Weather)
const FuelM = memo(Fuel)
const ElectronicsM = memo(Electronics)
const TireM = memo(Tire)
const LapHistoryM = memo(LapHistory)
const WindM = memo(Wind)
const EngineerTranscriptM = memo(EngineerTranscript)

export const REGISTRY: WidgetDef[] = [
  {
    id: 'telemetryInputs',
    title: 'Telemetry Inputs',
    render: (d) => <TelemetryInputsM snap={d.tel} />,
    default: { w: 4, h: 7, minW: 3, minH: 3 },
  },
  {
    id: 'standings',
    title: 'Standings',
    render: (d) => <StandingsM snap={d.standings} playerCarIdx={d.tel?.playerCarIdx ?? null} />,
    default: { w: 12, h: 9, minW: 3, minH: 3 },
  },
  {
    id: 'sof',
    title: 'Strength of Field',
    render: (d) => <SoFM snap={d.standings} playerCarIdx={d.tel?.playerCarIdx ?? null} />,
    default: { w: 4, h: 7, minW: 3, minH: 3 },
  },
  {
    id: 'trackMap',
    title: 'Track Map',
    render: (d) => <TrackMapM snap={d.trackMap} playerCarIdx={d.tel?.playerCarIdx ?? null} info={d.info} standings={d.standings} onDelete={d.onDeleteTrackMap} />,
    default: { w: 6, h: 12, minW: 3, minH: 3 },
  },
  {
    id: 'weather',
    title: 'Weather',
    render: (d) => <WeatherM snap={d.tel} info={d.info} />,
    default: { w: 4, h: 8, minW: 3, minH: 3 },
  },
  {
    id: 'fuel',
    title: 'Fuel',
    render: (d) => <FuelM snap={d.tel} />,
    default: { w: 4, h: 9, minW: 3, minH: 3 },
  },
  {
    id: 'electronics',
    title: 'Electronics',
    render: (d) => <ElectronicsM snap={d.tel} />,
    default: { w: 6, h: 10, minW: 3, minH: 3 },
  },
  {
    id: 'tire',
    title: 'Tires',
    render: (d) => <TireM snap={d.tel} />,
    default: { w: 6, h: 11, minW: 3, minH: 3 },
  },
  {
    id: 'lapHistory',
    title: 'Lap History',
    render: (d) => <LapHistoryM snap={d.tel} info={d.info} />,
    default: { w: 5, h: 9, minW: 3, minH: 3 },
  },
  {
    id: 'wind',
    title: 'Wind',
    render: (d) => <WindM snap={d.tel} info={d.info} />,
    default: { w: 4, h: 8, minW: 2, minH: 5 },
  },
  {
    id: 'engineerTranscript',
    title: 'Engineer Transcript',
    render: () => <EngineerTranscriptM />,
    default: { w: 4, h: 9, minW: 3, minH: 4 },
  },
]
