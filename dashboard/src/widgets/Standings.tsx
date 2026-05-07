import type { StandingsSnapshot } from '@shared/StandingsSnapshot'
import { fmtLapTime } from '../format'

interface Props {
  snap: StandingsSnapshot | null
  playerCarIdx: number | null
}

const fmtGap = (g: number | null) => (g == null ? '—' : g === 0 ? '—' : `+${g.toFixed(3)}`)
const fmtSec = (s: number) => s.toFixed(1) + 's'

function hexFromInt(n: number | bigint | null | undefined): string | null {
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

function ClassBadge({
  shortName,
  classPos,
  classColor,
}: {
  shortName: string
  classPos: number
  classColor: bigint | null
}) {
  const bg = hexFromInt(classColor) ?? '#444'
  const fg = readableTextOn(bg)
  return (
    <span className="class-badge" style={{ background: bg, color: fg }}>
      <span className="class-badge-cls">{shortName || '—'}</span>
      <span className="class-badge-pos">{classPos}</span>
    </span>
  )
}

function LicBadge({
  licString,
  licColor,
}: {
  licString: string
  licColor: bigint | null
}) {
  if (!licString) return <span>—</span>
  const [letter, ...rest] = licString.split(' ')
  const bg = hexFromInt(licColor) ?? '#444'
  const fg = readableTextOn(bg)
  return (
    <span className="lic-badge">
      <span className="lic-badge-letter" style={{ background: bg, color: fg }}>{letter}</span>
      <span className="lic-badge-sr">{rest.join(' ')}</span>
    </span>
  )
}

function sectorColor(
  last: number | undefined,
  pb: number | null | undefined,
  sessionBest: number | null | undefined,
): string | undefined {
  if (last == null || last <= 0) return undefined
  if (sessionBest != null && Math.abs(last - sessionBest) < 0.001) return '#a855f7'
  if (pb != null && Math.abs(last - pb) < 0.001) return '#22c55e'
  return undefined
}

export function Standings({ snap, playerCarIdx }: Props) {
  if (!snap) {
    return (
      <section className="card">
        <h2>Standings</h2>
        <p className="muted">waiting for first frame…</p>
      </section>
    )
  }

  const nSectors = snap.sessionBestSectors.length

  return (
    <section className="card">
      <h2>
        Standings <span className="muted">— {snap.sessionType} ({snap.entries.length})</span>
      </h2>
      <div className="card-body" style={{ overflowX: 'auto' }}>
      <table className="standings">
        <thead>
          <tr>
            <th>P</th>
            <th>Car</th>
            <th>#</th>
            <th>Driver</th>
            <th>Lap</th>
            <th>Last</th>
            <th>Best</th>
            <th>Gap</th>
            <th>Pit</th>
            <th>Inc</th>
            {Array.from({ length: nSectors }, (_, i) => (
              <th key={`s${i+1}h`} title={`Sector ${i+1} last / best`}>S{i+1}</th>
            ))}
            <th>iR</th>
            <th>SR</th>
          </tr>
        </thead>
        <tbody>
          {snap.entries.map((e, i) => {
            const prev = i > 0 ? snap.entries[i - 1] : null
            const classBoundary = prev != null && prev.carClassId !== e.carClassId
            const classes = [
              e.carIdx === playerCarIdx ? 'me' : '',
              classBoundary ? 'class-divider' : '',
            ]
              .filter(Boolean)
              .join(' ')
            return (
              <tr key={e.carIdx} className={classes}>
                <td>
                  <ClassBadge
                    shortName={e.carClassShortName}
                    classPos={e.classPosition}
                    classColor={e.carClassColor}
                  />
                </td>
                <td className="manufacturer">{e.manufacturer ?? '—'}</td>
                <td>{e.carNumber}</td>
                <td className="name">{e.userName}</td>
                <td>{e.lap}</td>
                <td>{fmtLapTime(e.lastLapTime)}</td>
                <td>{fmtLapTime(e.bestLapTime)}</td>
                <td>{fmtGap(e.gapToLeader)}</td>
                <td>
                  {e.currentPitRoadSec != null ? `● ${fmtSec(e.currentPitRoadSec)}` : ''}
                </td>
                <td>{e.incidents}</td>
                {Array.from({ length: nSectors }, (_, si) => {
                  const last = e.lastSectorTimes[si]
                  const pb   = e.bestSectorTimes[si]
                  const sb   = snap.sessionBestSectors[si]
                  const color = sectorColor(last, pb, sb)
                  return (
                    <td
                      key={`s${si}`}
                      title={`S${si+1}  last: ${last > 0 ? fmtLapTime(last) : '—'}  best: ${pb != null ? fmtLapTime(pb) : '—'}`}
                      style={color ? { color } : undefined}
                    >
                      {last > 0 ? fmtLapTime(last) : '—'}
                    </td>
                  )
                })}
                <td>
                  {e.irating > 0
                    ? <span className="ir-badge">{e.irating}</span>
                    : '—'}
                </td>
                <td>
                  <LicBadge licString={e.safetyRating} licColor={e.licColor} />
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
      </div>
    </section>
  )
}
