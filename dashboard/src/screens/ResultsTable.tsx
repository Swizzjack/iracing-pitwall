import { useState } from 'react'
import type { SessionSummary } from '@shared/SessionSummary'
import { fmtLapMs } from '../format'

type SortKey = 'startTime' | 'trackName' | 'eventTypeName' | 'playerFinishPosition' | 'sof'

const PHASE_LABEL: Record<string, string> = { P: 'Practice', Q: 'Qualify', R: 'Race' }

interface Props {
  sessions: SessionSummary[]
  onSelect: (subSessionId: number) => void
}

export function ResultsTable({ sessions, onSelect }: Props) {
  const [sort, setSort] = useState<{ key: SortKey; asc: boolean }>({ key: 'startTime', asc: false })

  function toggleSort(key: SortKey) {
    setSort((s) => (s.key === key ? { key, asc: !s.asc } : { key, asc: true }))
  }

  const sorted = [...sessions].sort((a, b) => {
    const av = a[sort.key]
    const bv = b[sort.key]
    if (av == null && bv == null) return 0
    if (av == null) return 1
    if (bv == null) return -1
    const cmp = av < bv ? -1 : av > bv ? 1 : 0
    return sort.asc ? cmp : -cmp
  })

  function arrow(key: SortKey) {
    if (sort.key !== key) return null
    return <span className="sort-arrow">{sort.asc ? '▲' : '▼'}</span>
  }

  function fmtDate(ms: number) {
    return new Date(ms).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
  }


  return (
    <div className="results-table-wrap">
      <table className="results-table">
        <thead>
          <tr>
            <th onClick={() => toggleSort('startTime')} className="sortable">Date {arrow('startTime')}</th>
            <th onClick={() => toggleSort('trackName')} className="sortable">Track {arrow('trackName')}</th>
            <th onClick={() => toggleSort('eventTypeName')} className="sortable">Type {arrow('eventTypeName')}</th>
            <th>Series</th>
            <th>Car</th>
            <th onClick={() => toggleSort('playerFinishPosition')} className="sortable col-num">Pos {arrow('playerFinishPosition')}</th>
            <th className="col-mono">Best Lap</th>
            <th onClick={() => toggleSort('sof')} className="sortable col-num">SoF {arrow('sof')}</th>
          </tr>
        </thead>
        <tbody>
          {sorted.map((s) => (
            <tr key={String(s.subSessionId)} onClick={() => onSelect(s.subSessionId)} className="results-row">
              <td>{fmtDate(s.startTime)}</td>
              <td>{s.trackConfig ? `${s.trackName} — ${s.trackConfig}` : (s.trackName ?? '—')}</td>
              <td>
                <span className="event-badge">{PHASE_LABEL[s.simsessionType] ?? s.simsessionType}</span>
                {!s.isFinalized && <span className="live-dot" title="Session nicht beendet"> ●</span>}
              </td>
              <td className="col-series">{s.seriesName ?? '—'}</td>
              <td>{s.playerCarName ?? '—'}</td>
              <td className="col-num">{s.playerFinishPosition != null ? `P${s.playerFinishPosition}` : '—'}</td>
              <td className="col-mono">{fmtLapMs(s.playerBestLapMs)}</td>
              <td className="col-num">{s.sof ?? '—'}</td>
            </tr>
          ))}
          {sorted.length === 0 && (
            <tr><td colSpan={8} className="no-results">No results</td></tr>
          )}
        </tbody>
      </table>
    </div>
  )
}
