import { useState } from 'react'
import type { SessionSummary } from '@shared/SessionSummary'

type SortKey = 'startTime' | 'trackName' | 'eventTypeName' | 'playerFinishPosition' | 'playerIncidents' | 'playerOldiRating' | 'sof'

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

  function fmtLap(ms: number | null) {
    if (ms == null || ms <= 0) return '—'
    const s = ms / 1000
    const m = Math.floor(s / 60)
    return `${m}:${(s % 60).toFixed(3).padStart(6, '0')}`
  }

  function iRatingDelta(s: SessionSummary) {
    const { playerOldiRating: old, playerNewiRating: nw } = s
    if (old == null || nw == null) return null
    const d = nw - old
    return <span className={d > 0 ? 'delta-pos' : d < 0 ? 'delta-neg' : 'delta-zero'}>{d > 0 ? '+' : ''}{d}</span>
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
            <th onClick={() => toggleSort('playerFinishPosition')} className="sortable">Pos {arrow('playerFinishPosition')}</th>
            <th onClick={() => toggleSort('playerIncidents')} className="sortable">Inc {arrow('playerIncidents')}</th>
            <th>Best Lap</th>
            <th onClick={() => toggleSort('playerOldiRating')} className="sortable">iR {arrow('playerOldiRating')}</th>
            <th onClick={() => toggleSort('sof')} className="sortable">SoF {arrow('sof')}</th>
          </tr>
        </thead>
        <tbody>
          {sorted.map((s) => (
            <tr key={s.subSessionId} onClick={() => onSelect(s.subSessionId)} className="results-row">
              <td>{fmtDate(s.startTime)}</td>
              <td>{s.trackConfig ? `${s.trackName} — ${s.trackConfig}` : (s.trackName ?? '—')}</td>
              <td><span className="event-badge">{s.eventTypeName ?? '—'}</span></td>
              <td className="col-series">{s.seriesName ?? '—'}</td>
              <td>{s.playerCarName ?? '—'}</td>
              <td className="col-num">{s.playerFinishPosition != null ? `P${s.playerFinishPosition + 1}` : '—'}</td>
              <td className={`col-num ${(s.playerIncidents ?? 0) >= 4 ? 'inc-warn' : ''}`}>{s.playerIncidents ?? '—'}</td>
              <td className="col-mono">{fmtLap(s.playerBestLapMs)}</td>
              <td className="col-num">
                {s.playerOldiRating ?? '—'}
                {' '}{iRatingDelta(s)}
              </td>
              <td className="col-num">{s.sof ?? '—'}</td>
            </tr>
          ))}
          {sorted.length === 0 && (
            <tr><td colSpan={10} className="no-results">No results</td></tr>
          )}
        </tbody>
      </table>
    </div>
  )
}
