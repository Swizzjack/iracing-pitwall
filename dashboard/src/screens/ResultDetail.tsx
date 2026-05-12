import type { SessionDetail } from '@shared/SessionDetail'
import type { DriverResult } from '@shared/DriverResult'

interface Props {
  detail: SessionDetail
  onClose: () => void
}

export function ResultDetail({ detail, onClose }: Props) {
  const { summary, results } = detail

  function fmtDate(ms: number) {
    return new Date(ms).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })
  }

  function fmtLap(ms: number | null) {
    if (ms == null || ms <= 0) return '—'
    const s = ms / 1000
    const m = Math.floor(s / 60)
    return `${m}:${(s % 60).toFixed(3).padStart(6, '0')}`
  }

  function srDelta(r: DriverResult) {
    if (r.oldsr == null || r.newsr == null) return null
    const d = (r.newsr - r.oldsr) / 100
    return <span className={d > 0 ? 'delta-pos' : d < 0 ? 'delta-neg' : 'delta-zero'}>{d > 0 ? '+' : ''}{d.toFixed(2)}</span>
  }

  // Group by simsession_type
  const sessionTypes = Array.from(new Set(results.map((r) => r.simsessionType))).sort()
  const labelMap: Record<string, string> = { P: 'Practice', Q: 'Qualifying', R: 'Race' }

  return (
    <div className="result-detail">
      <div className="result-detail-header">
        <div>
          <h2>{summary.trackConfig ? `${summary.trackName} — ${summary.trackConfig}` : summary.trackName}</h2>
          <div className="result-detail-meta">
            <span>{summary.seriesName}</span>
            <span>{summary.eventTypeName}</span>
            <span>{fmtDate(summary.startTime)}</span>
            {summary.sof && <span>SoF {summary.sof}</span>}
          </div>
        </div>
        <button className="header-btn" onClick={onClose}>✕ Close</button>
      </div>

      {sessionTypes.map((type) => {
        const rows = results.filter((r) => r.simsessionType === type)
        return (
          <div key={type} className="result-session-block">
            <h3>{labelMap[type] ?? type}</h3>
            <div className="results-table-wrap">
              <table className="results-table">
                <thead>
                  <tr>
                    <th>Pos</th>
                    <th>Driver</th>
                    <th>Car</th>
                    <th>Laps</th>
                    <th>Best Lap</th>
                    <th>Avg Lap</th>
                    <th>Inc</th>
                    <th>iR</th>
                    <th>SR</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((r) => (
                    <tr key={r.custId} className={r.isPlayer ? 'player-row' : ''}>
                      <td className="col-num">{r.finishPosition != null ? r.finishPosition + 1 : '—'}</td>
                      <td>{r.displayName ?? '—'}</td>
                      <td>{r.carName ?? '—'}</td>
                      <td className="col-num">{r.lapsComplete ?? '—'}</td>
                      <td className="col-mono">{fmtLap(r.bestLapMs)}</td>
                      <td className="col-mono">{fmtLap(r.averageLapMs)}</td>
                      <td className={`col-num ${(r.incidents ?? 0) >= 4 ? 'inc-warn' : ''}`}>{r.incidents ?? '—'}</td>
                      <td className="col-num">
                        {r.oldiRating ?? '—'}
                        {r.oldiRating != null && r.newiRating != null && (
                          <span className={(r.newiRating - r.oldiRating) > 0 ? 'delta-pos' : 'delta-neg'}>
                            {' '}{r.newiRating - r.oldiRating > 0 ? '+' : ''}{r.newiRating - r.oldiRating}
                          </span>
                        )}
                      </td>
                      <td className="col-num">{srDelta(r)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )
      })}
    </div>
  )
}
