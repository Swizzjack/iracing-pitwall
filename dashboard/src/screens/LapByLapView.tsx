import type { LapRow } from '@shared/LapRow'
import { fmtLapTime, parseSectors } from '../format'

interface Props {
  laps: LapRow[]
  driverName: string
  carNumber: string
  onClose: () => void
}


function fmtSec(s: number | null): string {
  if (s == null) return '—'
  return s.toFixed(3)
}

export function LapByLapView({ laps, driverName, carNumber, onClose }: Props) {
  // Sort by lap number
  const sorted = [...laps].sort((a, b) => a.lapNum - b.lapNum)
  const hasS2 = sorted.some((l) => parseSectors(l.sectorsJson).length >= 2)
  const hasS3 = sorted.some((l) => parseSectors(l.sectorsJson).length >= 3)
  const hasAirTemp = sorted.some((l) => l.airTemp != null)
  const hasTrackTemp = sorted.some((l) => l.trackTemp != null)

  // Best valid lap time for highlighting
  const bestTime = sorted
    .filter((l) => l.valid && l.lapTimeSec != null && l.lapTimeSec > 0)
    .map((l) => l.lapTimeSec!)
    .reduce((a, b) => Math.min(a, b), Infinity)

  return (
    <div className="lap-by-lap">
      <div className="lap-by-lap-header">
        <div>
          <span className="lap-car-num">#{carNumber}</span>
          <span className="lap-driver-name">{driverName}</span>
        </div>
        <button className="header-btn" onClick={onClose}>✕ Close laps</button>
      </div>

      <div className="results-table-wrap">
        <table className="results-table lap-table">
          <thead>
            <tr>
              <th className="col-num">Lap</th>
              <th className="col-mono">Time</th>
              <th className="col-mono">S1</th>
              {hasS2 && <th className="col-mono">S2</th>}
              {hasS3 && <th className="col-mono">S3</th>}
              <th className="col-num">Valid</th>
              <th className="col-num">In-Lap</th>
              {hasAirTemp && <th className="col-num">Air °C</th>}
              {hasTrackTemp && <th className="col-num">Track °C</th>}
            </tr>
          </thead>
          <tbody>
            {sorted.map((lap) => {
              const sectors = parseSectors(lap.sectorsJson)
              const isBest = lap.valid && lap.lapTimeSec === bestTime && bestTime < Infinity
              return (
                <tr key={lap.lapNum} className={isBest ? 'lap-best' : lap.valid ? '' : 'lap-invalid'}>
                  <td className="col-num">{lap.lapNum}</td>
                  <td className="col-mono">{fmtLapTime(lap.lapTimeSec)}</td>
                  <td className="col-mono">{fmtSec(sectors[0] ?? null)}</td>
                  {hasS2 && <td className="col-mono">{fmtSec(sectors[1] ?? null)}</td>}
                  {hasS3 && <td className="col-mono">{fmtSec(sectors[2] ?? null)}</td>}
                  <td className="col-num">{lap.valid ? '✓' : '✗'}</td>
                  <td className="col-num">{lap.inLap ? 'PIT' : '—'}</td>
                  {hasAirTemp && <td className="col-num">{lap.airTemp != null ? lap.airTemp.toFixed(1) : '—'}</td>}
                  {hasTrackTemp && <td className="col-num">{lap.trackTemp != null ? lap.trackTemp.toFixed(1) : '—'}</td>}
                </tr>
              )
            })}
            {sorted.length === 0 && (
              <tr><td colSpan={7} className="no-results">No lap data</td></tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}
