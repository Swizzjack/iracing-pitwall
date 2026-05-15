import { useState, useMemo } from 'react'
import type { SessionDetail } from '@shared/SessionDetail'
import type { DriverResult } from '@shared/DriverResult'
import type { LapRow } from '@shared/LapRow'
import { LapByLapView } from './LapByLapView'
import { LapTimeChart } from './charts/LapTimeChart'
import { fmtLapMs, parseSectors, hexFromClassColor, readableTextOn, LIC_COLORS } from '../format'

type Tab = 'results' | 'charts'

type SortKey =
  | 'finishPosition' | 'classPosition' | 'carNumber' | 'displayName'
  | 'carName' | 'carClassName' | 'lapsComplete' | 'bestLapMs'
  | 's1' | 's2' | 's3' | 'incidents' | 'pitStops' | 'iRating'

interface Props {
  detail: SessionDetail
  laps: LapRow[] | null
  lapsCarIdx: number | null
  chartLaps: LapRow[] | null
  onClose: () => void
  onLoadLaps: (subSessionId: number, carIdx: number) => void
  onCloseLaps: () => void
  onLoadChartLaps: (subSessionId: number) => void
}

function fmtDate(ms: number) {
  return new Date(ms).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })
}

function fmtSec(s: number): string {
  return s.toFixed(3)
}

function RatingBadge({ r }: { r: DriverResult }) {
  const sr = r.safetyRating ?? ''
  const ir = r.oldiRating
  if (!sr && ir == null) return <span className="rating-val">—</span>

  const cls = sr.charAt(0).toUpperCase()
  const { bg, fg } = LIC_COLORS[cls] ?? { bg: 'rgba(30,30,30,0.82)', fg: '#aaa' }

  // iR delta for API-sourced rows
  let delta: React.ReactNode = null
  if (r.source !== 'live' && ir != null && r.newiRating != null && r.newiRating !== ir) {
    const d = r.newiRating - ir
    delta = (
      <span className={d > 0 ? 'delta-pos' : 'delta-neg'} style={{ marginLeft: 4 }}>
        {d > 0 ? '+' : ''}{d}
      </span>
    )
  }

  // SR delta for API-sourced rows
  let srDelta: React.ReactNode = null
  if (r.source !== 'live' && r.oldsr != null && r.newsr != null && r.newsr !== r.oldsr) {
    const d = (r.newsr - r.oldsr) / 100
    srDelta = (
      <span className={d > 0 ? 'delta-pos' : 'delta-neg'} style={{ marginLeft: 4 }}>
        {d > 0 ? '+' : ''}{d.toFixed(2)}
      </span>
    )
  }

  const parts = [sr || null, ir != null && ir > 0 ? String(ir) : null].filter(Boolean)
  return (
    <span style={{ display: 'inline-flex', alignItems: 'center', gap: 2, whiteSpace: 'nowrap' }}>
      <span className="rating-badge" style={{ background: bg, color: fg }}>
        {parts.join(' ') || '—'}
      </span>
      {delta}
      {srDelta}
    </span>
  )
}

// ── Sorting ───────────────────────────────────────────────────────────────────

function hasTime(r: DriverResult): boolean {
  return r.bestLapMs != null && r.bestLapMs > 0
}

function compareRows(
  a: DriverResult, b: DriverResult,
  key: SortKey, asc: boolean,
  aSecs: number[], bSecs: number[],
): number {
  // Drivers without a recorded lap time always sort to the bottom
  const at = hasTime(a), bt = hasTime(b)
  if (at !== bt) return at ? -1 : 1

  const sign = asc ? 1 : -1

  // Missing numeric values sort last regardless of direction
  function numCmp(av: number, bv: number): number {
    const aInf = !isFinite(av), bInf = !isFinite(bv)
    if (aInf && bInf) return 0
    if (aInf) return 1
    if (bInf) return -1
    return sign * (av - bv)
  }

  switch (key) {
    case 'finishPosition':
      return numCmp(
        a.finishPosition != null && a.finishPosition > 0 ? a.finishPosition : Infinity,
        b.finishPosition != null && b.finishPosition > 0 ? b.finishPosition : Infinity,
      )
    case 'classPosition':
      return numCmp(
        a.classPosition != null && a.classPosition > 0 ? a.classPosition : Infinity,
        b.classPosition != null && b.classPosition > 0 ? b.classPosition : Infinity,
      )
    case 'carNumber': {
      const an = parseInt(a.carNumber ?? '', 10)
      const bn = parseInt(b.carNumber ?? '', 10)
      if (!isNaN(an) && !isNaN(bn)) return sign * (an - bn)
      return sign * (a.carNumber ?? '').localeCompare(b.carNumber ?? '')
    }
    case 'displayName':
      return sign * (a.displayName ?? '').localeCompare(b.displayName ?? '')
    case 'carName':
      return sign * (a.carName ?? '').localeCompare(b.carName ?? '')
    case 'carClassName':
      return sign * (a.carClassName ?? '').localeCompare(b.carClassName ?? '')
    case 'lapsComplete':
      return numCmp(a.lapsComplete ?? Infinity, b.lapsComplete ?? Infinity)
    case 'bestLapMs':
      return numCmp(
        a.bestLapMs != null && a.bestLapMs > 0 ? a.bestLapMs : Infinity,
        b.bestLapMs != null && b.bestLapMs > 0 ? b.bestLapMs : Infinity,
      )
    case 's1': return numCmp(aSecs[0] ?? Infinity, bSecs[0] ?? Infinity)
    case 's2': return numCmp(aSecs[1] ?? Infinity, bSecs[1] ?? Infinity)
    case 's3': return numCmp(aSecs[2] ?? Infinity, bSecs[2] ?? Infinity)
    case 'incidents':
      return numCmp(a.incidents ?? Infinity, b.incidents ?? Infinity)
    case 'pitStops':
      return numCmp(a.pitStops ?? Infinity, b.pitStops ?? Infinity)
    case 'iRating':
      return numCmp(
        a.oldiRating != null && a.oldiRating > 0 ? a.oldiRating : Infinity,
        b.oldiRating != null && b.oldiRating > 0 ? b.oldiRating : Infinity,
      )
    default:
      return 0
  }
}

// ── Per-session sortable table ────────────────────────────────────────────────

interface SessionTableProps {
  rows: DriverResult[]
  isMultiClass: boolean
  useClassPos: boolean
  maxSectors: number
  subSessionId: number
  onLoadLaps: (subSessionId: number, carIdx: number) => void
}

function SessionResultsTable({
  rows, isMultiClass, useClassPos, maxSectors, subSessionId, onLoadLaps,
}: SessionTableProps) {
  const defaultKey: SortKey = useClassPos ? 'classPosition' : 'finishPosition'
  const [sort, setSort] = useState<{ key: SortKey; asc: boolean }>({ key: defaultKey, asc: true })

  function toggleSort(key: SortKey) {
    setSort((s) => (s.key === key ? { key, asc: !s.asc } : { key, asc: true }))
  }

  function arrow(key: SortKey) {
    if (sort.key !== key) return null
    return <span className="sort-arrow">{sort.asc ? '▲' : '▼'}</span>
  }

  const sortedRows = useMemo(() => {
    const sectorsMap = new Map(rows.map((r) => [r.carIdx, parseSectors(r.lastSectorsJson)]))
    return [...rows].sort((a, b) =>
      compareRows(a, b, sort.key, sort.asc, sectorsMap.get(a.carIdx) ?? [], sectorsMap.get(b.carIdx) ?? [])
    )
  }, [rows, sort])

  const posKey: SortKey = useClassPos ? 'classPosition' : 'finishPosition'

  return (
    <div className="results-table-wrap">
      <table className="results-table">
        <thead>
          <tr>
            <th className="sortable col-num" onClick={() => toggleSort(posKey)}>
              Pos {arrow(posKey)}
            </th>
            {isMultiClass && !useClassPos && (
              <th className="sortable" onClick={() => toggleSort('carClassName')}>
                Class {arrow('carClassName')}
              </th>
            )}
            <th className="sortable col-num" onClick={() => toggleSort('carNumber')}># {arrow('carNumber')}</th>
            <th className="sortable" onClick={() => toggleSort('displayName')}>Driver {arrow('displayName')}</th>
            <th className="sortable" onClick={() => toggleSort('carName')}>Car {arrow('carName')}</th>
            <th className="sortable col-num" onClick={() => toggleSort('lapsComplete')}>Laps {arrow('lapsComplete')}</th>
            <th className="sortable col-mono" onClick={() => toggleSort('bestLapMs')}>Best Lap {arrow('bestLapMs')}</th>
            {maxSectors >= 1 && <th className="sortable col-mono" onClick={() => toggleSort('s1')}>S1 {arrow('s1')}</th>}
            {maxSectors >= 2 && <th className="sortable col-mono" onClick={() => toggleSort('s2')}>S2 {arrow('s2')}</th>}
            {maxSectors >= 3 && <th className="sortable col-mono" onClick={() => toggleSort('s3')}>S3 {arrow('s3')}</th>}
            <th className="sortable col-num" onClick={() => toggleSort('incidents')}>Inc {arrow('incidents')}</th>
            <th className="sortable col-num" onClick={() => toggleSort('pitStops')}>Pit {arrow('pitStops')}</th>
            <th className="sortable" onClick={() => toggleSort('iRating')}>Rating {arrow('iRating')}</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {sortedRows.map((r) => {
            const sectors = parseSectors(r.lastSectorsJson)
            const pos = useClassPos ? r.classPosition : r.finishPosition
            const classBg = hexFromClassColor(r.carClassColor) ?? '#444'
            const classFg = readableTextOn(classBg)
            return (
              <tr key={r.carIdx} className={r.isPlayer ? 'player-row' : ''}>
                <td className="col-num">{pos != null && pos > 0 ? `P${pos}` : '—'}</td>
                {isMultiClass && !useClassPos && (
                  <td>
                    <span className="class-pip" style={{ background: classBg, color: classFg }}>
                      {r.carClassName ?? '?'}
                    </span>
                  </td>
                )}
                <td className="col-num" style={{ color: '#888' }}>{r.carNumber ?? '—'}</td>
                <td>{r.displayName ?? '—'}</td>
                <td>{r.carName ?? '—'}</td>
                <td className="col-num">{r.lapsComplete ?? '—'}</td>
                <td className="col-mono">{fmtLapMs(r.bestLapMs)}</td>
                {maxSectors >= 1 && <td className="col-mono">{sectors[0] != null ? fmtSec(sectors[0]) : '—'}</td>}
                {maxSectors >= 2 && <td className="col-mono">{sectors[1] != null ? fmtSec(sectors[1]) : '—'}</td>}
                {maxSectors >= 3 && <td className="col-mono">{sectors[2] != null ? fmtSec(sectors[2]) : '—'}</td>}
                <td className={`col-num ${(r.incidents ?? 0) >= 4 ? 'inc-warn' : ''}`}>{r.incidents ?? '—'}</td>
                <td className="col-num">{r.pitStops ?? '—'}</td>
                <td><RatingBadge r={r} /></td>
                <td>
                  <button
                    className="lap-detail-btn"
                    onClick={() => onLoadLaps(subSessionId, r.carIdx)}
                    title="Lap-by-Lap"
                  >⏱</button>
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}

// ── Main component ────────────────────────────────────────────────────────────

export function ResultDetail({
  detail,
  laps,
  lapsCarIdx,
  chartLaps,
  onClose,
  onLoadLaps,
  onCloseLaps,
  onLoadChartLaps,
}: Props) {
  const [tab, setTab] = useState<Tab>('results')
  const [selectedClass, setSelectedClass] = useState<string | null>(null)
  const { summary, results } = detail

  const sessionTypes = useMemo(
    () => Array.from(new Set(results.map((r) => r.simsessionType))).sort(),
    [results],
  )
  const labelMap: Record<string, string> = { P: 'Practice', Q: 'Qualifying', R: 'Race' }

  const maxSectors = useMemo(
    () => results.reduce((max, r) => Math.max(max, parseSectors(r.lastSectorsJson).length), 0),
    [results],
  )

  const primaryType = sessionTypes.includes('R') ? 'R' : sessionTypes[0] ?? 'R'

  const allClasses = useMemo(() => {
    const seen = new Map<string, { name: string; color: number | null }>()
    for (const r of results) {
      const name = r.carClassName ?? ''
      if (name && !seen.has(name)) {
        seen.set(name, { name, color: r.carClassColor ?? null })
      }
    }
    return Array.from(seen.values())
  }, [results])

  const isMultiClass = allClasses.length > 1

  // Per-driver lap view takes full screen — early return AFTER all hooks
  if (laps !== null && lapsCarIdx !== null) {
    const driver = results.find((r) => r.carIdx === lapsCarIdx)
    return (
      <LapByLapView
        laps={laps}
        driverName={driver?.displayName ?? `Car ${lapsCarIdx}`}
        carNumber={driver?.carNumber ?? String(lapsCarIdx)}
        onClose={onCloseLaps}
      />
    )
  }

  function handleTabChange(next: Tab) {
    setTab(next)
    if (next === 'charts' && chartLaps === null) {
      onLoadChartLaps(summary.subSessionId)
    }
  }

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
            {!summary.isFinalized && (
              <span className="source-badge source-live" title="Session läuft noch">LIVE</span>
            )}
          </div>
        </div>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <div className="detail-tabs">
            <button
              className={`detail-tab${tab === 'results' ? ' detail-tab-active' : ''}`}
              onClick={() => handleTabChange('results')}
            >Results</button>
            <button
              className={`detail-tab${tab === 'charts' ? ' detail-tab-active' : ''}`}
              onClick={() => handleTabChange('charts')}
            >Charts</button>
          </div>
          <button className="header-btn" onClick={onClose}>✕ Close</button>
        </div>
      </div>

      {tab === 'charts' && (
        <div className="detail-charts">
          {chartLaps === null ? (
            <div className="chart-empty">Loading lap data…</div>
          ) : (
            <LapTimeChart
              laps={chartLaps}
              results={results.filter((r) => r.simsessionType === primaryType)}
              simsessionType={primaryType}
            />
          )}
        </div>
      )}

      {tab === 'results' && (
        <>
          {isMultiClass && (
            <div className="class-filter-bar">
              <button
                className={`class-filter-btn${selectedClass === null ? ' class-filter-active' : ''}`}
                onClick={() => setSelectedClass(null)}
              >All</button>
              {allClasses.map(({ name, color }) => {
                const bg = hexFromClassColor(color) ?? '#444'
                const fg = readableTextOn(bg)
                const active = selectedClass === name
                return (
                  <button
                    key={name}
                    className={`class-filter-btn${active ? ' class-filter-active' : ''}`}
                    style={active ? { background: bg, color: fg, borderColor: bg } : {}}
                    onClick={() => setSelectedClass(active ? null : name)}
                  >
                    <span className="class-filter-dot" style={{ background: bg }} />
                    {name}
                  </button>
                )
              })}
            </div>
          )}

          {sessionTypes.map((type) => {
            const allRows = results.filter((r) => r.simsessionType === type)
            const rows = selectedClass != null
              ? allRows.filter((r) => r.carClassName === selectedClass)
              : allRows
            const useClassPos = selectedClass != null && isMultiClass
            return (
              <div key={type} className="result-session-block">
                <h3>{labelMap[type] ?? type}</h3>
                <SessionResultsTable
                  rows={rows}
                  isMultiClass={isMultiClass}
                  useClassPos={useClassPos}
                  maxSectors={maxSectors}
                  subSessionId={summary.subSessionId}
                  onLoadLaps={onLoadLaps}
                />
              </div>
            )
          })}
        </>
      )}
    </div>
  )
}
