import { useMemo } from 'react'
import type { StandingsSnapshot } from '@shared/StandingsSnapshot'

type Props = { snap: StandingsSnapshot | null; playerCarIdx: number | null }

const BR = 1600 / Math.LN2
const BIN_SIZE = 500
const SVG_W = 200
const SVG_H = 44

type BinColor = '#f5a623' | '#a86c14' | '#3a3a3a'

type ClassCard = {
  classId: number
  name: string
  sof: number | null
  count: number
  min: number
  max: number
  rangeStart: number
  rangeEnd: number
  bins: number[]
  binColors: BinColor[]
  markerX: number | null
}

function buildCard(
  classId: number,
  name: string,
  ratings: number[],
): ClassCard {
  const count = ratings.length
  if (count === 0) {
    return { classId, name, sof: null, count: 0, min: 0, max: 0, rangeStart: 0, rangeEnd: 0, bins: [], binColors: [], markerX: null }
  }

  const min = Math.min(...ratings)
  const max = Math.max(...ratings)
  const rangeStart = Math.floor(min / BIN_SIZE) * BIN_SIZE
  const rawEnd = Math.ceil(max / BIN_SIZE) * BIN_SIZE
  const rangeEnd = rawEnd === rangeStart ? rangeStart + BIN_SIZE : rawEnd
  const binCount = (rangeEnd - rangeStart) / BIN_SIZE

  const bins: number[] = Array.from({ length: binCount }, (_, i) => {
    const lo = rangeStart + i * BIN_SIZE
    const hi = lo + BIN_SIZE
    return ratings.filter(r => (i < binCount - 1 ? r >= lo && r < hi : r >= lo && r <= hi)).length
  })

  const top1 = Math.max(...bins)
  const top2 = Math.max(...bins.filter(c => c < top1), -1)

  const binColors: BinColor[] = bins.map(c => {
    if (c === top1 && c > 0) return '#f5a623'
    if (c === top2 && c > 0) return '#a86c14'
    return '#3a3a3a'
  })

  const sof = BR * Math.log(count / ratings.reduce((s, r) => s + Math.pow(2, -r / 1600), 0))

  const markerX = binCount > 0
    ? ((sof - rangeStart) / (rangeEnd - rangeStart)) * SVG_W
    : null

  return { classId, name, sof, count, min, max, rangeStart, rangeEnd, bins, binColors, markerX }
}

function Histogram({ card }: { card: ClassCard }) {
  if (card.bins.length === 0 || card.sof == null) return null

  const maxCount = Math.max(...card.bins, 1)
  const binW = SVG_W / card.bins.length
  const padT = 4
  const padB = 4
  const chartH = SVG_H - padT - padB
  const baseY = padT + chartH
  const mx = card.markerX ?? 0

  return (
    <svg
      className="sof-histogram"
      viewBox={`0 0 ${SVG_W} ${SVG_H}`}
      preserveAspectRatio="none"
      aria-hidden="true"
    >
      {card.bins.map((cnt, i) => {
        const h = cnt > 0 ? Math.max(2, (cnt / maxCount) * chartH) : 0
        return (
          <rect
            key={i}
            x={i * binW + 1}
            y={baseY - h}
            width={binW - 2}
            height={h}
            style={{ fill: card.binColors[i] }}
          />
        )
      })}
      <line className="sof-baseline" x1={0} y1={baseY} x2={SVG_W} y2={baseY} />
      <line className="sof-marker" x1={mx} y1={padT} x2={mx} y2={baseY} />
    </svg>
  )
}

function ClassCardView({ card }: { card: ClassCard }) {
  return (
    <article className="sof-class">
      <header className="sof-header">
        <span className="sof-title">Strength of Field</span>
        <span className="sof-meta">
          <span className="sof-meta-value">{card.count}</span>
          <span className="sof-meta-label"> Drivers{card.count > 0 ? ' · ' : ''}</span>
          {card.count > 0 && (
            <span className="sof-meta-value">{card.min}–{card.max}</span>
          )}
        </span>
      </header>
      <div className="sof-body">
        <span className="sof-class-badge">{card.name || '—'}</span>
        <Histogram card={card} />
        <div className="sof-score">
          <div className="sof-value">
            {card.sof != null ? Math.round(card.sof).toLocaleString() : '—'}
          </div>
          <div className="sof-caption">SOF</div>
        </div>
      </div>
    </article>
  )
}

export function SoF({ snap, playerCarIdx }: Props) {
  const cards = useMemo<ClassCard[]>(() => {
    if (!snap || playerCarIdx == null) return []

    const playerClassId = snap.entries.find(e => e.carIdx === playerCarIdx)?.carClassId ?? null
    if (playerClassId == null) return []

    const byClass = new Map<number, { name: string; ratings: number[] }>()
    for (const e of snap.entries) {
      if (!byClass.has(e.carClassId)) {
        byClass.set(e.carClassId, {
          name: e.carClassShortName ?? String(e.carClassId),
          ratings: [],
        })
      }
      if (e.irating > 0) byClass.get(e.carClassId)!.ratings.push(e.irating)
    }

    const all = [...byClass.entries()]
      .map(([classId, { name, ratings }]) => buildCard(classId, name, ratings))
      .sort((a, b) => {
        if (a.classId === playerClassId) return -1
        if (b.classId === playerClassId) return 1
        if (a.sof == null && b.sof == null) return 0
        if (a.sof == null) return 1
        if (b.sof == null) return -1
        return b.sof - a.sof
      })

    return all
  }, [snap, playerCarIdx])

  if (!snap || playerCarIdx == null || cards.length === 0) {
    return (
      <section className="card">
        <h2>Strength of Field</h2>
        <p className="muted">waiting for first frame…</p>
      </section>
    )
  }

  return (
    <section className="card">
      <h2>Strength of Field</h2>
      <div className="card-body sof-stack">
        {cards.map(card => (
          <ClassCardView key={card.classId} card={card} />
        ))}
      </div>
    </section>
  )
}
