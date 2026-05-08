import { useMemo } from 'react'
import type { StandingsSnapshot } from '@shared/StandingsSnapshot'

type Props = { snap: StandingsSnapshot | null }

const BR = 1600 / Math.LN2

function hexFromInt(n: bigint | null): string | null {
  if (n == null) return null
  const v = typeof n === 'bigint' ? Number(n) : n
  const c = Math.max(0, Math.min(0xffffff, v))
  return `#${c.toString(16).padStart(6, '0')}`
}

function readableTextOn(bg: string): string {
  const r = parseInt(bg.slice(1, 3), 16)
  const g = parseInt(bg.slice(3, 5), 16)
  const b = parseInt(bg.slice(5, 7), 16)
  const luma = (0.299 * r + 0.587 * g + 0.114 * b) / 255
  return luma > 0.55 ? '#000' : '#fff'
}

type ClassRow = {
  classId: number
  name: string
  color: bigint | null
  sof: number | null
  count: number
  min: number
  max: number
}

export function SoF({ snap }: Props) {
  const rows = useMemo<ClassRow[]>(() => {
    if (!snap) return []
    const byClass = new Map<number, { name: string; color: bigint | null; ratings: number[] }>()
    for (const e of snap.entries) {
      if (!byClass.has(e.carClassId)) {
        byClass.set(e.carClassId, {
          name: e.carClassShortName ?? String(e.carClassId),
          color: e.carClassColor,
          ratings: [],
        })
      }
      if (e.irating > 0) {
        byClass.get(e.carClassId)!.ratings.push(e.irating)
      }
    }
    return [...byClass.entries()]
      .map(([classId, { name, color, ratings }]) => {
        const count = ratings.length
        const sof =
          count > 0
            ? BR * Math.log(count / ratings.reduce((s, r) => s + Math.pow(2, -r / 1600), 0))
            : null
        const min = count > 0 ? Math.min(...ratings) : 0
        const max = count > 0 ? Math.max(...ratings) : 0
        return { classId, name, color, sof, count, min, max }
      })
      .sort((a, b) => {
        if (a.sof == null && b.sof == null) return 0
        if (a.sof == null) return 1
        if (b.sof == null) return -1
        return b.sof - a.sof
      })
  }, [snap])

  if (!snap) {
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
      <div className="card-body">
        {rows.map(row => {
          const bg = hexFromInt(row.color) ?? '#444'
          const fg = readableTextOn(bg)
          return (
            <div key={row.classId} className="sof-row">
              <span className="class-badge sof-class-badge" style={{ background: bg, color: fg }}>
                {row.name || '—'}
              </span>
              <div className="big-stat">
                <div className="big-value">
                  {row.sof != null ? String(Math.round(row.sof)) : '—'}
                </div>
                <div className="big-label">SoF</div>
              </div>
              <dl className="kv">
                <dt>drivers</dt>
                <dd>{row.count}</dd>
                <dt>range</dt>
                <dd>{row.count > 0 ? `${row.min}–${row.max}` : '—'}</dd>
              </dl>
            </div>
          )
        })}
      </div>
    </section>
  )
}
