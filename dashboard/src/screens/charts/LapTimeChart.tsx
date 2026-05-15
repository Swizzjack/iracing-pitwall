import { useMemo } from 'react'
import type { LapRow } from '@shared/LapRow'
import type { DriverResult } from '@shared/DriverResult'
import { UPlotChart } from './UPlotChart'

interface Props {
  laps: LapRow[]
  results: DriverResult[]
  simsessionType?: string
}

function fmtLapTime(v: number): string {
  const m = Math.floor(v / 60)
  const s = v % 60
  return `${m}:${s.toFixed(3).padStart(6, '0')}`
}

// Muted color palette for non-player cars
const PALETTE = [
  '#3b7dd8', '#e05c3a', '#3dba6b', '#d4a017', '#9b59b6',
  '#1abc9c', '#e74c3c', '#2ecc71', '#f39c12', '#8e44ad',
]

export function LapTimeChart({ laps, results, simsessionType = 'R' }: Props) {
  const playerCarIdx = results.find((r) => r.isPlayer && r.simsessionType === simsessionType)?.carIdx ?? -1

  const { data, series } = useMemo(() => {
    // Filter to the target session type and valid non-in-laps with a real time
    const relevant = laps.filter(
      (l) =>
        l.simsessionNum != null &&
        l.valid &&
        !l.inLap &&
        l.lapTimeSec != null &&
        l.lapTimeSec > 0,
    )

    if (relevant.length === 0) return { data: [[]] as uPlot.AlignedData, series: [] }

    // Collect all unique lap numbers and sort
    const lapNums = Array.from(new Set(relevant.map((l) => l.lapNum))).sort((a, b) => a - b)

    // Group by car_idx
    const byCarMap = new Map<number, Map<number, number>>()
    for (const l of relevant) {
      if (!byCarMap.has(l.carIdx)) byCarMap.set(l.carIdx, new Map())
      byCarMap.get(l.carIdx)!.set(l.lapNum, l.lapTimeSec!)
    }

    // Put player first, then sort others by finish position
    const playerFirst = playerCarIdx !== -1 ? [playerCarIdx] : []
    const others = [...byCarMap.keys()]
      .filter((idx) => idx !== playerCarIdx)
      .sort((a, b) => {
        const pa = results.find((r) => r.carIdx === a)?.finishPosition ?? 9999
        const pb = results.find((r) => r.carIdx === b)?.finishPosition ?? 9999
        return pa - pb
      })
    const orderedCars = [...playerFirst, ...others]

    // Build aligned data arrays
    const xArr: number[] = lapNums
    const yArrs: (number | null)[][] = orderedCars.map((carIdx) => {
      const times = byCarMap.get(carIdx)!
      return lapNums.map((n) => times.get(n) ?? null)
    })

    const alignedData: uPlot.AlignedData = [xArr, ...yArrs]

    // Compute series options
    let colorIdx = 0
    const seriesDefs: uPlot.Series[] = orderedCars.map((carIdx) => {
      const isPlayer = carIdx === playerCarIdx
      const dr = results.find((r) => r.carIdx === carIdx)
      const label = dr ? (dr.displayName ?? `#${dr.carNumber ?? carIdx}`) : `Car ${carIdx}`
      return {
        label,
        stroke: isPlayer ? '#facc15' : PALETTE[colorIdx++ % PALETTE.length],
        width: isPlayer ? 2 : 1,
        points: { show: false },
        alpha: isPlayer ? 1 : 0.6,
        value: (_u: uPlot, v: number | null) => v == null ? '—' : fmtLapTime(v),
      }
    })

    return { data: alignedData, series: seriesDefs }
  }, [laps, results, playerCarIdx, simsessionType])

  const options = useMemo(
    (): Omit<uPlot.Options, 'width' | 'height'> => ({
      cursor: { sync: { key: 'laptime' } },
      legend: { show: series.length <= 6 },
      scales: {
        x: { time: false },
        y: {
          range: (_u, min, max) => {
            const pad = (max - min) * 0.05 || 1
            return [Math.max(0, min - pad), max + pad]
          },
        },
      },
      axes: [
        {
          label: 'Lap',
          stroke: '#666',
          grid: { stroke: '#1e1e1e', width: 1 },
          ticks: { stroke: '#333', width: 1 },
          values: (_u, ticks) => ticks.map((t) => String(Math.round(t))),
        },
        {
          label: 'Lap time',
          stroke: '#666',
          grid: { stroke: '#1e1e1e', width: 1 },
          ticks: { stroke: '#333', width: 1 },
          values: (_u, ticks) => ticks.map((t) => fmtLapTime(t)),
          size: 80,
        },
      ],
      series: [{ label: 'Lap', value: (_u: uPlot, v: number | null) => v == null ? '—' : String(Math.round(v)) }, ...series],
    }),
    [series],
  )

  if (series.length === 0) {
    return <div className="chart-empty">No valid lap data available</div>
  }

  return (
    <div className="chart-block">
      <div className="chart-title">Lap Times</div>
      <UPlotChart key={series.length} data={data} options={options} height={280} />
    </div>
  )
}
