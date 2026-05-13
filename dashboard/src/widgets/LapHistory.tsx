import { useState } from 'react'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import type { SessionInfoYaml } from '@shared/SessionInfoYaml'
import { SettingsDrawer } from '../components/SettingsDrawer'
import { LapHistorySettings, type SortOrder } from './LapHistorySettings'
import { useLapHistory, type LapRecord } from './useLapHistory'

// ─── Formatters ───────────────────────────────────────────────────────────────

function fmtTime(sec: number): string {
  if (sec <= 0) return '--:--.---'
  const m = Math.floor(sec / 60)
  const s = sec % 60
  return `${m}:${s.toFixed(3).padStart(6, '0')}`
}

function fmtTemp(c: number): string {
  return `${c.toFixed(1)}°`
}

// ─── Persistence ──────────────────────────────────────────────────────────────

const PREFS_KEY = 'iracing-laphistory-prefs-v1'

interface Prefs {
  sortOrder: SortOrder
  fontScale: number
}

function loadPrefs(): Prefs {
  try {
    const raw = localStorage.getItem(PREFS_KEY)
    if (raw) {
      const p = JSON.parse(raw) as Partial<Prefs>
      return {
        sortOrder: (['newest', 'oldest', 'fastest'] as SortOrder[]).includes(p.sortOrder as SortOrder)
          ? p.sortOrder as SortOrder
          : 'newest',
        fontScale: typeof p.fontScale === 'number' ? p.fontScale : 1,
      }
    }
  } catch { /* ignore */ }
  return { sortOrder: 'newest', fontScale: 1 }
}

function savePrefs(p: Prefs) {
  try { localStorage.setItem(PREFS_KEY, JSON.stringify(p)) } catch { /* ignore */ }
}

// ─── Sorted view ──────────────────────────────────────────────────────────────

function sortedHistory(history: LapRecord[], order: SortOrder): LapRecord[] {
  if (order === 'newest') return [...history].reverse()
  if (order === 'oldest') return [...history]
  // fastest: sort valid laps first by time, then invalid at end
  return [...history].sort((a, b) => {
    if (a.valid && !b.valid) return -1
    if (!a.valid && b.valid) return 1
    return a.lapTimeSec - b.lapTimeSec
  })
}

// ─── Row component ────────────────────────────────────────────────────────────

function LapRow({
  record,
  isBest,
}: {
  record: LapRecord
  isBest: boolean
  fontScale: number
}) {
  const isSpecial = record.isInLap || record.isOutLap || !record.valid
  const baseColor = isBest ? '#facc15' : isSpecial ? '#555' : '#ccc'
  const timeColor = isBest ? '#facc15' : isSpecial ? '#555' : '#e5e7eb'

  const badge = record.isInLap
    ? 'IN'
    : record.isOutLap
      ? 'OUT'
      : !record.valid
        ? 'INV'
        : null

  const wetnessLabel = record.trackWetness != null && record.trackWetness > 0
    ? ['', 'Dry', 'M.Dry', 'V.Lt', 'Lt', 'Mod', 'Wet', 'St.Wt'][record.trackWetness] ?? `${record.trackWetness}`
    : null

  const rubberShort = record.trackRubberState
    ? record.trackRubberState.replace(/\bmoderate(ly)?\b/i, 'mod').replace(/\busage\b/i, '').trim()
    : null

  const fs = (px: number) => `calc(${px}px * var(--widget-font-scale, 1))`

  return (
    <tr style={{ opacity: isSpecial ? 0.55 : 1 }}>
      <td style={{ color: baseColor, fontFamily: 'var(--font-mono)', fontSize: fs(12), padding: '3px 6px', textAlign: 'right', whiteSpace: 'nowrap' }}>
        {isBest && <span style={{ color: '#facc15', marginRight: 3 }}>★</span>}
        L{record.lapNumber}
        {badge && (
          <span style={{
            marginLeft: 4, fontSize: fs(10), fontWeight: 700,
            background: '#1f1f1f', borderRadius: 3, padding: '0 4px',
            color: record.isInLap || record.isOutLap ? '#f97316' : '#555',
          }}>
            {badge}
          </span>
        )}
      </td>
      <td style={{ color: timeColor, fontFamily: 'var(--font-mono)', fontWeight: isBest ? 700 : 400, fontSize: fs(13), padding: '3px 6px', textAlign: 'right' }}>
        {fmtTime(record.lapTimeSec)}
      </td>
      <td style={{ color: '#60a5fa', fontFamily: 'var(--font-mono)', fontSize: fs(11), padding: '3px 6px', textAlign: 'right' }}>
        {fmtTemp(record.airTemp)}
      </td>
      <td style={{ color: '#f97316', fontFamily: 'var(--font-mono)', fontSize: fs(11), padding: '3px 6px', textAlign: 'right' }}>
        {fmtTemp(record.trackTemp)}
      </td>
      <td style={{ color: '#a78bfa', fontSize: fs(10), padding: '3px 6px', maxWidth: 80, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
        {rubberShort ?? '–'}
      </td>
      {wetnessLabel != null && (
        <td style={{ color: '#38bdf8', fontSize: fs(10), padding: '3px 6px', whiteSpace: 'nowrap' }}>
          {wetnessLabel}
        </td>
      )}
    </tr>
  )
}

// ─── Main widget ──────────────────────────────────────────────────────────────

interface Props {
  snap: TelemetrySnapshot | null
  info: SessionInfoYaml | null
}

export function LapHistory({ snap, info }: Props) {
  const [showSettings, setShowSettings] = useState(false)
  const [prefs, setPrefs] = useState<Prefs>(loadPrefs)

  function handleSortOrder(sortOrder: SortOrder) {
    const next = { ...prefs, sortOrder }
    setPrefs(next)
    savePrefs(next)
  }

  function handleFontScale(fontScale: number) {
    const next = { ...prefs, fontScale }
    setPrefs(next)
    savePrefs(next)
  }

  const { history, reset } = useLapHistory(snap, info)

  if (!snap) {
    return (
      <section className="card">
        <h2>Lap History</h2>
        <div className="card-body">
          <p className="muted">waiting for first frame…</p>
        </div>
      </section>
    )
  }

  const validLaps = history.filter(r => r.valid)
  const bestTime = validLaps.length > 0 ? Math.min(...validLaps.map(r => r.lapTimeSec)) : null

  const displayed = sortedHistory(history, prefs.sortOrder)
  const hasWetness = history.some(r => r.trackWetness != null && r.trackWetness > 0)

  return (
    <section className="card" style={{ '--widget-font-scale': prefs.fontScale } as React.CSSProperties}>
      <h2 style={{ display: 'flex', alignItems: 'center' }}>
        Lap History
        <button
          title="Reset lap history"
          onClick={reset}
          style={{
            marginLeft: 'auto', background: 'none', border: 'none', color: '#555',
            cursor: 'pointer', fontSize: 14, padding: '0 6px', lineHeight: 1,
          }}
        >↺</button>
        <button
          className={`header-btn${showSettings ? ' header-btn-active' : ''}`}
          style={{ marginLeft: 4 }}
          onClick={() => setShowSettings(s => !s)}
        >⚙</button>
      </h2>

      <SettingsDrawer open={showSettings} onClose={() => setShowSettings(false)} title="Lap History Settings">
        <LapHistorySettings
          sortOrder={prefs.sortOrder}
          fontScale={prefs.fontScale}
          onSortOrder={handleSortOrder}
          onFontScale={handleFontScale}
        />
      </SettingsDrawer>

      <div className="card-body" style={{ padding: '4px 0 8px', overflowY: 'auto' }}>
        {history.length === 0 ? (
          <p className="muted" style={{ padding: '0 16px' }}>No laps recorded yet.</p>
        ) : (
          <table style={{ width: '100%', borderCollapse: 'collapse', tableLayout: 'fixed' }}>
            <colgroup>
              <col style={{ width: '18%' }} />
              <col style={{ width: '26%' }} />
              <col style={{ width: '14%' }} />
              <col style={{ width: '14%' }} />
              <col style={{ width: hasWetness ? '18%' : '28%' }} />
              {hasWetness && <col style={{ width: '10%' }} />}
            </colgroup>
            <thead>
              <tr style={{ borderBottom: '1px solid #1a1a1a' }}>
                {['Lap', 'Time', 'Air', 'Trk', 'Rubber', ...(hasWetness ? ['Wet'] : [])].map(h => (
                  <th key={h} style={{
                    color: '#444', fontSize: `calc(10px * var(--widget-font-scale, 1))`,
                    fontWeight: 600, letterSpacing: '0.05em', padding: '2px 6px',
                    textAlign: h === 'Lap' ? 'right' : h === 'Time' ? 'right' : h === 'Air' || h === 'Trk' ? 'right' : 'left',
                  }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {displayed.map((record, i) => (
                <LapRow
                  key={`${record.lapNumber}-${i}`}
                  record={record}
                  isBest={bestTime !== null && record.lapTimeSec === bestTime && record.valid}
                  fontScale={prefs.fontScale}
                />
              ))}
            </tbody>
          </table>
        )}
      </div>
    </section>
  )
}
