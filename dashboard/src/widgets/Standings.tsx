import { useState, useEffect, useMemo, useRef, useCallback } from 'react'
import type { StandingsSnapshot } from '@shared/StandingsSnapshot'
import type { StandingEntry } from '@shared/StandingEntry'
import { fmtLapTime, hexFromClassColor, readableTextOn, sofFromRatings, LIC_COLORS } from '../format'
import { SettingsDrawer } from '../components/SettingsDrawer'
import { StandingsSettings } from './StandingsSettings'
import type { ColId } from './StandingsSettings'

interface Props {
  snap: StandingsSnapshot | null
  playerCarIdx: number | null
}

const DEFAULT_ORDER: ColId[] = ['pos', 'car', 'num', 'driver', 'lap', 'last', 'best', 'gap', 'pit', 'tire', 'p2p', 'sectors', 'rating']
const TH_LABELS: Record<ColId, string> = {
  pos: 'P', car: 'Car', num: '#', driver: 'Driver', lap: 'Lap',
  last: 'Last', best: 'Best', gap: 'Gap', pit: 'Pit', tire: 'TC', p2p: 'P2P', p2p_cooldown: 'Cooldown',
  sectors: '', rating: 'Rating',
}

const LS_KEY = 'iracing-standings-columns'
const SCHEMA_VERSION = 3

type SortDir = 'asc' | 'desc'

interface Persisted {
  v: number
  order: ColId[]
  hidden: ColId[]
  widths: Partial<Record<ColId, number>>
  fontScale: number
  sortBy: ColId | null
  sortDir: SortDir
  splitP2PCooldown: boolean
}

function loadConfig(): Persisted {
  const defaults: Persisted = {
    v: SCHEMA_VERSION,
    order: [...DEFAULT_ORDER],
    hidden: [],
    widths: {},
    fontScale: 1,
    sortBy: null,
    sortDir: 'asc',
    splitP2PCooldown: false,
  }
  try {
    const raw = localStorage.getItem(LS_KEY)
    if (!raw) return defaults
    const parsed = JSON.parse(raw) as Partial<Persisted> & { order?: string[], hidden?: string[] }
    let order = (parsed.order ?? DEFAULT_ORDER) as ColId[]
    const hiddenArr = (parsed.hidden ?? []) as ColId[]

    // Migrate legacy ir/sr columns → single rating column
    const hasIr = order.includes('ir' as ColId)
    const hasSr = order.includes('sr' as ColId)
    if (hasIr || hasSr) {
      const firstIdx = Math.min(
        hasIr ? order.indexOf('ir' as ColId) : Infinity,
        hasSr ? order.indexOf('sr' as ColId) : Infinity,
      )
      order = order.filter(id => id !== ('ir' as ColId) && id !== ('sr' as ColId))
      order.splice(firstIdx, 0, 'rating')
      const irHidden = hiddenArr.includes('ir' as ColId)
      const srHidden = hiddenArr.includes('sr' as ColId)
      const filteredHidden = hiddenArr.filter(id => id !== ('ir' as ColId) && id !== ('sr' as ColId))
      if (irHidden && srHidden) filteredHidden.push('rating')
    }

    // Migrate inc → tire
    if (order.includes('inc' as ColId)) {
      const incIdx = order.indexOf('inc' as ColId)
      order = order.filter(id => id !== ('inc' as ColId))
      order.splice(incIdx, 0, 'tire')
    }
    const hiddenFiltered = hiddenArr.filter(id => id !== ('inc' as ColId))

    const known = new Set<ColId>(DEFAULT_ORDER)
    const safeOrder = order.filter(id => known.has(id))
    DEFAULT_ORDER.forEach(id => { if (!safeOrder.includes(id)) safeOrder.push(id) })

    return {
      v: SCHEMA_VERSION,
      order: safeOrder,
      hidden: hiddenFiltered.filter(id => known.has(id)),
      widths: (parsed.widths ?? {}) as Partial<Record<ColId, number>>,
      fontScale: parsed.fontScale ?? 1,
      sortBy: parsed.sortBy ?? null,
      sortDir: parsed.sortDir ?? 'asc',
      splitP2PCooldown: parsed.splitP2PCooldown ?? false,
    }
  } catch {
    return defaults
  }
}

const fmtGap = (g: number | null) => {
  if (g == null || g === 0) return '—'
  // Negative integer-like values encode laps-down (iRacing ResultsPositions.Time).
  if (g < 0 && Math.abs(g - Math.round(g)) < 0.01) return `+${Math.abs(Math.round(g))}L`
  return `+${g.toFixed(3)}`
}
const fmtSec = (s: number) => s.toFixed(1) + 's'
const fmtP2P = (s: number) => s.toFixed(1)


const TIRE_COMPOUNDS: Record<number, { label: string; bg: string }> = {
  0: { label: 'Dry',  bg: '#52525b' },
  1: { label: 'Wet',  bg: '#2563eb' },
}

function TireBadge({ compound }: { compound: number | null }) {
  if (compound == null) return null
  const known = TIRE_COMPOUNDS[compound]
  const bg  = known?.bg ?? '#334155'
  const fg  = readableTextOn(bg)
  const txt = known?.label ?? String(compound)
  return <span className="rating-badge" style={{ background: bg, color: fg }}>{txt}</span>
}

function ClassBadge({ classPos, classColor }: {
  classPos: number
  classColor: bigint | null
}) {
  const bg = hexFromClassColor(classColor) ?? '#444'
  const fg = readableTextOn(bg)
  return (
    <span className="class-badge" style={{ background: bg, color: fg }}>
      <span className="class-badge-pos">{classPos > 0 ? classPos : '—'}</span>
    </span>
  )
}


function RatingBadge({ irating, safetyRating }: {
  irating: number; safetyRating: string
}) {
  if (irating <= 0 && !safetyRating) return <span>—</span>
  const cls = safetyRating.charAt(0).toUpperCase()
  const { bg, fg } = LIC_COLORS[cls] ?? { bg: 'rgba(30, 30, 30, 0.82)', fg: '#aaaaaa' }
  const parts = [safetyRating || null, irating > 0 ? String(irating) : null].filter(Boolean)
  return (
    <span className="rating-badge" style={{ background: bg, color: fg }}>
      {parts.length ? parts.join(' ') : '—'}
    </span>
  )
}

function sectorColor(last: number | undefined, pb: number | null | undefined, sessionBest: number | null | undefined): string | undefined {
  if (last == null || last <= 0) return undefined
  if (sessionBest != null && Math.abs(last - sessionBest) < 0.001) return '#a855f7'
  if (pb != null && Math.abs(last - pb) < 0.001) return '#22c55e'
  return undefined
}

// Comparators per column for sorting. Nulls/invalids sorted to end (desc = ascending from end).
function compareEntries(a: StandingEntry, b: StandingEntry, col: ColId, dir: SortDir): number {
  const sign = dir === 'asc' ? 1 : -1
  switch (col) {
    case 'pos': {
      const av = a.position > 0 ? a.position : Infinity
      const bv = b.position > 0 ? b.position : Infinity
      return sign * (av - bv)
    }
    case 'car': return sign * (a.manufacturer ?? '').localeCompare(b.manufacturer ?? '')
    case 'num': return sign * (a.carNumber).localeCompare(b.carNumber, undefined, { numeric: true })
    case 'driver': return sign * a.userName.localeCompare(b.userName)
    case 'lap': return sign * (a.lap - b.lap)
    case 'last': {
      const av = a.lastLapTime > 0 ? a.lastLapTime : Infinity
      const bv = b.lastLapTime > 0 ? b.lastLapTime : Infinity
      return sign * (av - bv)
    }
    case 'best': {
      const av = a.bestLapTime > 0 ? a.bestLapTime : Infinity
      const bv = b.bestLapTime > 0 ? b.bestLapTime : Infinity
      return sign * (av - bv)
    }
    case 'gap': {
      const av = a.gapToLeader ?? Infinity
      const bv = b.gapToLeader ?? Infinity
      return sign * (av - bv)
    }
    case 'pit': return sign * ((a.pitStops ?? 0) - (b.pitStops ?? 0))
    case 'tire': return sign * ((a.tireCompound ?? -1) - (b.tireCompound ?? -1))
    case 'p2p': {
      const av = a.p2pRemaining ?? Infinity
      const bv = b.p2pRemaining ?? Infinity
      return sign * (av - bv)
    }
    case 'p2p_cooldown': {
      const av = a.p2pCooldown ?? Infinity
      const bv = b.p2pCooldown ?? Infinity
      return sign * (av - bv)
    }
    case 'rating': return sign * (a.irating - b.irating)
    case 'sectors': return 0
    default: return 0
  }
}

export function Standings({ snap, playerCarIdx }: Props) {
  const [config, setConfig] = useState<Persisted>(loadConfig)
  const [showSettings, setShowSettings] = useState(false)
  const resizingRef = useRef<{ id: ColId; startX: number; startWidth: number } | null>(null)

  const { order: colOrder, hidden: hiddenArr, widths, fontScale, sortBy, sortDir } = config
  const hiddenCols = useMemo(() => new Set(hiddenArr), [hiddenArr])

  useEffect(() => {
    try {
      localStorage.setItem(LS_KEY, JSON.stringify(config))
    } catch { /* localStorage unavailable */ }
  }, [config])

  const sessionBestLap = useMemo(() => {
    if (!snap) return null
    const ts = snap.entries.filter(e => e.bestLapTime > 0).map(e => e.bestLapTime)
    return ts.length ? Math.min(...ts) : null
  }, [snap])

  const classBestMap = useMemo(() => {
    const m = new Map<number, number>()
    if (!snap) return m
    for (const e of snap.entries) {
      if (e.bestLapTime > 0) {
        const c = m.get(e.carClassId)
        if (c == null || e.bestLapTime < c) m.set(e.carClassId, e.bestLapTime)
      }
    }
    return m
  }, [snap])

  const sortedEntries = useMemo(() => {
    if (!snap) return []
    if (!sortBy) return snap.entries
    return [...snap.entries].sort((a, b) => compareEntries(a, b, sortBy, sortDir))
  }, [snap, sortBy, sortDir])

  const sofByClass = useMemo(() => {
    if (!snap) return []
    const byClass = new Map<number, { name: string; color: string | null; ratings: number[] }>()
    for (const e of snap.entries) {
      if (!byClass.has(e.carClassId))
        byClass.set(e.carClassId, { name: e.carClassShortName, color: hexFromClassColor(e.carClassColor), ratings: [] })
      if (e.irating > 0) byClass.get(e.carClassId)!.ratings.push(e.irating)
    }
    const playerClassId = playerCarIdx != null
      ? snap.entries.find(e => e.carIdx === playerCarIdx)?.carClassId ?? null
      : null
    return [...byClass.entries()]
      .map(([classId, { name, color, ratings }]) => {
        const sof = sofFromRatings(ratings)
        if (sof == null) return null
        return { classId, name, color, sof: Math.round(sof) }
      })
      .filter((x): x is { classId: number; name: string; color: string | null; sof: number } => x != null)
      .sort((a, b) => {
        if (a.classId === playerClassId) return -1
        if (b.classId === playerClassId) return 1
        return b.sof - a.sof
      })
  }, [snap, playerCarIdx])

  function handleHeaderClick(id: ColId) {
    if (id === 'sectors') return
    setConfig(prev => {
      if (prev.sortBy !== id) return { ...prev, sortBy: id, sortDir: 'asc' }
      if (prev.sortDir === 'asc') return { ...prev, sortDir: 'desc' }
      return { ...prev, sortBy: null, sortDir: 'asc' }
    })
  }

  const handleResizeStart = useCallback((id: ColId, e: React.MouseEvent<HTMLSpanElement>) => {
    e.preventDefault()
    const th = (e.currentTarget as HTMLElement).closest('th') as HTMLElement
    resizingRef.current = { id, startX: e.clientX, startWidth: th.offsetWidth }

    function onMove(ev: MouseEvent) {
      const r = resizingRef.current
      if (!r) return
      const newW = Math.max(40, r.startWidth + (ev.clientX - r.startX))
      setConfig(prev => ({ ...prev, widths: { ...prev.widths, [r.id]: newW } }))
    }
    function onUp() {
      resizingRef.current = null
      window.removeEventListener('mousemove', onMove)
      window.removeEventListener('mouseup', onUp)
    }
    window.addEventListener('mousemove', onMove)
    window.addEventListener('mouseup', onUp)
  }, [])

  function bestLapColor(e: StandingEntry): string | undefined {
    if (e.bestLapTime <= 0) return undefined
    if (sessionBestLap != null && Math.abs(e.bestLapTime - sessionBestLap) < 0.001) return '#a855f7'
    const cb = classBestMap.get(e.carClassId)
    if (cb != null && Math.abs(e.bestLapTime - cb) < 0.001) return '#22c55e'
    return undefined
  }

  if (!snap) {
    return (
      <section className="card">
        <h2>Standings</h2>
        <p className="muted">waiting for first frame…</p>
      </section>
    )
  }

  const visibleCols = colOrder.filter(id => !hiddenCols.has(id))
  if (config.splitP2PCooldown) {
    const p2pIdx = visibleCols.indexOf('p2p')
    if (p2pIdx !== -1) visibleCols.splice(p2pIdx + 1, 0, 'p2p_cooldown')
  }
  const nSectors = snap.sessionBestSectors.length

  function thCells(id: ColId) {
    const sortAttr = sortBy === id ? sortDir : undefined
    if (id === 'sectors') {
      return Array.from({ length: nSectors }, (_, i) => (
        <th key={`s${i + 1}h`} title={`Sector ${i + 1} last / best`}>S{i + 1}</th>
      ))
    }
    return [
      <th
        key={id}
        data-sort={sortAttr}
        onClick={() => handleHeaderClick(id)}
        style={widths[id] != null ? { width: widths[id] } : undefined}
      >
        {TH_LABELS[id]}
        <span
          className="col-resize-handle"
          onMouseDown={ev => handleResizeStart(id, ev)}
          onClick={e => e.stopPropagation()}
        />
      </th>
    ]
  }

  function tdCells(id: ColId, e: StandingEntry) {
    switch (id) {
      case 'pos': return [
        <td key="pos">
          <ClassBadge classPos={e.classPosition} classColor={e.carClassColor} />
        </td>
      ]
      case 'car': return [<td key="car" className="manufacturer">{e.manufacturer ?? '—'}</td>]
      case 'num': return [<td key="num">{e.carNumber}</td>]
      case 'driver': return [<td key="driver" className="name">{e.userName}</td>]
      case 'lap': return [<td key="lap">{e.lap < 0 ? '' : e.lap}</td>]
      case 'last': return [<td key="last">{fmtLapTime(e.lastLapTime)}</td>]
      case 'best': {
        const color = bestLapColor(e)
        return [<td key="best" style={color ? { color } : undefined}>{fmtLapTime(e.bestLapTime)}</td>]
      }
      case 'gap': return [<td key="gap">{fmtGap(e.gapToLeader)}</td>]
      case 'pit': return [
        <td key="pit">{e.currentPitRoadSec != null ? `● ${fmtSec(e.currentPitRoadSec)}` : ''}</td>
      ]
      case 'tire': return [<td key="tire"><TireBadge compound={e.tireCompound} /></td>]
      case 'p2p': {
        if (config.splitP2PCooldown) {
          const style = e.p2pActive ? { color: '#22c55e', fontWeight: 600 } : undefined
          const text = e.p2pRemaining != null ? fmtP2P(e.p2pRemaining) : '—'
          return [<td key="p2p" style={style}>{text}</td>]
        }
        const style = e.p2pActive
          ? { color: '#22c55e', fontWeight: 600 }
          : e.p2pCooldown != null
            ? { color: '#ef4444', fontWeight: 600 }
            : undefined
        const text = e.p2pCooldown != null
          ? fmtP2P(e.p2pCooldown)
          : e.p2pRemaining != null ? fmtP2P(e.p2pRemaining) : '—'
        return [
          <td key="p2p" title={e.p2pCooldown != null ? 'P2P cooldown' : undefined} style={style}>{text}</td>
        ]
      }
      case 'p2p_cooldown': {
        const style = e.p2pCooldown != null ? { color: '#ef4444', fontWeight: 600 } : undefined
        const text = e.p2pCooldown != null ? fmtP2P(e.p2pCooldown) : '—'
        return [<td key="p2p_cooldown" title="P2P cooldown" style={style}>{text}</td>]
      }
      case 'sectors': return Array.from({ length: nSectors }, (_, si) => {
        const last = e.lastSectorTimes[si]
        const pb = e.bestSectorTimes[si]
        const sb = snap!.sessionBestSectors[si]
        const color = sectorColor(last, pb, sb)
        return (
          <td
            key={`s${si}`}
            title={`S${si + 1}  last: ${last > 0 ? fmtLapTime(last) : '—'}  best: ${pb != null ? fmtLapTime(pb) : '—'}`}
            style={color ? { color } : undefined}
          >
            {last > 0 ? fmtLapTime(last) : '—'}
          </td>
        )
      })
      case 'rating': return [
        <td key="rating">
          <RatingBadge irating={e.irating} safetyRating={e.safetyRating} />
        </td>
      ]
    }
  }

  const sofNode = sofByClass.length === 0 ? null : (
    <span className="standings-sof">
      {'SoF '}
      {sofByClass.map((c, i) => (
        <span key={c.classId}>
          {i > 0 && <span className="sof-sep"> · </span>}
          <span style={{ color: c.color ?? '#888' }}>{c.name}</span>
          {' '}
          <span className="sof-val">{c.sof}</span>
        </span>
      ))}
    </span>
  )

  return (
    <section className="card" style={{ '--widget-font-scale-local': fontScale } as React.CSSProperties}>
      <h2 style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <span>Standings <span className="muted">— {snap.sessionType} ({snap.entries.length})</span></span>
        {sofNode}
        <button
          className={`header-btn${showSettings ? ' header-btn-active' : ''}`}
          onClick={() => setShowSettings(s => !s)}
          title="Column settings"
          style={{ marginLeft: 'auto' }}
        >⚙</button>
      </h2>

      <SettingsDrawer
        open={showSettings}
        onClose={() => setShowSettings(false)}
        title="Standings settings"
      >
        <StandingsSettings
          order={colOrder}
          hidden={hiddenCols}
          widths={widths}
          fontScale={fontScale}
          splitP2PCooldown={config.splitP2PCooldown}
          onOrderChange={order => setConfig(prev => ({ ...prev, order }))}
          onToggle={id => setConfig(prev => {
            const hidden = prev.hidden.includes(id)
              ? prev.hidden.filter(h => h !== id)
              : [...prev.hidden, id]
            return { ...prev, hidden }
          })}
          onResetWidth={id => setConfig(prev => {
            const widths = { ...prev.widths }
            delete widths[id]
            return { ...prev, widths }
          })}
          onResetAllWidths={() => setConfig(prev => ({ ...prev, widths: {} }))}
          onFontScaleChange={fontScale => setConfig(prev => ({ ...prev, fontScale }))}
          onSplitP2PCooldownChange={splitP2PCooldown => setConfig(prev => ({ ...prev, splitP2PCooldown }))}
          onResetAll={() => setConfig({
            v: SCHEMA_VERSION,
            order: [...DEFAULT_ORDER],
            hidden: [],
            widths: {},
            fontScale: 1,
            sortBy: null,
            sortDir: 'asc',
            splitP2PCooldown: false,
          })}
        />
      </SettingsDrawer>

      <div className="card-body" style={{ overflowX: 'auto' }}>
        <table className="standings" style={{ tableLayout: Object.keys(widths).length > 0 ? 'fixed' : 'auto' }}>
          {Object.keys(widths).length > 0 && (
            <colgroup>
              {visibleCols.map(id => {
                if (id === 'sectors') {
                  return Array.from({ length: nSectors }, (_, i) => <col key={`sc${i}`} />)
                }
                return <col key={id} style={widths[id] != null ? { width: widths[id] } : undefined} />
              })}
            </colgroup>
          )}
          <thead>
            <tr>{visibleCols.flatMap(thCells)}</tr>
          </thead>
          <tbody>
            {sortedEntries.map((e, i) => {
              const prev = i > 0 ? sortedEntries[i - 1] : null
              const classBoundary = prev != null && prev.carClassId !== e.carClassId
              const cls = [e.carIdx === playerCarIdx ? 'me' : '', classBoundary ? 'class-divider' : '']
                .filter(Boolean).join(' ')
              return (
                <tr key={e.carIdx} className={cls}>
                  {visibleCols.flatMap(id => tdCells(id, e))}
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>
    </section>
  )
}
