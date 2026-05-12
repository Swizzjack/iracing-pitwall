import type { ReactNode } from 'react'
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

export type WidgetData = {
  tel: TelemetrySnapshot | null
  standings: StandingsSnapshot | null
  info: SessionInfoYaml | null
  trackMap: TrackMapSnapshot | null
}

export type WidgetDef = {
  id: string
  title: string
  render: (data: WidgetData) => ReactNode
  default: { w: number; h: number; minW: number; minH: number }
}

export const REGISTRY: WidgetDef[] = [
  {
    id: 'telemetryInputs',
    title: 'Telemetry Inputs',
    render: (d) => <TelemetryInputs snap={d.tel} />,
    default: { w: 4, h: 7, minW: 3, minH: 3 },
  },
  {
    id: 'standings',
    title: 'Standings',
    render: (d) => <Standings snap={d.standings} playerCarIdx={d.tel?.playerCarIdx ?? null} />,
    default: { w: 12, h: 9, minW: 3, minH: 3 },
  },
  {
    id: 'sof',
    title: 'Strength of Field',
    render: (d) => <SoF snap={d.standings} playerCarIdx={d.tel?.playerCarIdx ?? null} />,
    default: { w: 4, h: 7, minW: 3, minH: 3 },
  },
  {
    id: 'trackMap',
    title: 'Track Map',
    render: (d) => <TrackMap snap={d.trackMap} playerCarIdx={d.tel?.playerCarIdx ?? null} />,
    default: { w: 6, h: 12, minW: 3, minH: 3 },
  },
  {
    id: 'weather',
    title: 'Weather',
    render: (d) => <Weather snap={d.tel} info={d.info} />,
    default: { w: 4, h: 8, minW: 3, minH: 3 },
  },
  {
    id: 'fuel',
    title: 'Fuel',
    render: (d) => <Fuel snap={d.tel} />,
    default: { w: 4, h: 9, minW: 3, minH: 3 },
  },
  {
    id: 'electronics',
    title: 'Electronics',
    render: (d) => <Electronics snap={d.tel} />,
    default: { w: 6, h: 10, minW: 3, minH: 3 },
  },
  {
    id: 'tire',
    title: 'Tires',
    render: (d) => <Tire snap={d.tel} />,
    default: { w: 6, h: 11, minW: 3, minH: 3 },
  },
]
