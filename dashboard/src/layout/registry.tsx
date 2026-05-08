import type { ReactNode } from 'react'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import type { StandingsSnapshot } from '@shared/StandingsSnapshot'
import type { SessionInfoYaml } from '@shared/SessionInfoYaml'
import type { TrackMapSnapshot } from '@shared/TrackMapSnapshot'
import { Telemetry } from '../widgets/Telemetry'
import { SessionInfo } from '../widgets/SessionInfo'
import { Standings } from '../widgets/Standings'
import { TelemetryInputs } from '../widgets/TelemetryInputs'
import { TrackMap } from '../widgets/TrackMap'
import { SoF } from '../widgets/SoF'
import { Weather } from '../widgets/Weather'

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
    id: 'telemetry',
    title: 'Telemetry',
    render: (d) => <Telemetry snap={d.tel} />,
    default: { w: 4, h: 9, minW: 3, minH: 5 },
  },
  {
    id: 'session',
    title: 'Session',
    render: (d) => <SessionInfo info={d.info} />,
    default: { w: 4, h: 7, minW: 3, minH: 4 },
  },
  {
    id: 'telemetryInputs',
    title: 'Telemetry Inputs',
    render: (d) => <TelemetryInputs snap={d.tel} />,
    default: { w: 4, h: 7, minW: 3, minH: 4 },
  },
  {
    id: 'standings',
    title: 'Standings',
    render: (d) => <Standings snap={d.standings} playerCarIdx={d.tel?.playerCarIdx ?? null} />,
    default: { w: 12, h: 9, minW: 6, minH: 5 },
  },
  {
    id: 'sof',
    title: 'Strength of Field',
    render: (d) => <SoF snap={d.standings} />,
    default: { w: 4, h: 7, minW: 3, minH: 3 },
  },
  {
    id: 'trackMap',
    title: 'Track Map',
    render: (d) => <TrackMap snap={d.trackMap} playerCarIdx={d.tel?.playerCarIdx ?? null} />,
    default: { w: 6, h: 12, minW: 4, minH: 6 },
  },
  {
    id: 'weather',
    title: 'Weather',
    render: (d) => <Weather snap={d.tel} info={d.info} />,
    default: { w: 4, h: 8, minW: 3, minH: 5 },
  },
]
