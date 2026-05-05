import type { StandingsSnapshot } from '@shared/StandingsSnapshot'
import { fmtLapTime } from '../format'

interface Props {
  snap: StandingsSnapshot | null
  playerCarIdx: number | null
}

const fmtGap = (g: number | null) => (g == null ? '—' : g === 0 ? '—' : `+${g.toFixed(3)}`)
const fmtSec = (s: number) => s.toFixed(1) + 's'

export function Standings({ snap, playerCarIdx }: Props) {
  if (!snap) {
    return (
      <section className="card">
        <h2>Standings</h2>
        <p className="muted">waiting for first frame…</p>
      </section>
    )
  }

  return (
    <section className="card">
      <h2>
        Standings <span className="muted">— {snap.sessionType} ({snap.entries.length})</span>
      </h2>
      <table className="standings">
        <thead>
          <tr>
            <th>P</th>
            <th>CP</th>
            <th>Cls</th>
            <th>#</th>
            <th>Driver</th>
            <th>Lap</th>
            <th>Last</th>
            <th>Best</th>
            <th>Gap</th>
            <th>Pit</th>
            <th>Inc</th>
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
                <td>{e.position}</td>
                <td>{e.classPosition}</td>
                <td className="cls">{e.carClassShortName}</td>
                <td>{e.carNumber}</td>
                <td className="name">{e.userName}</td>
                <td>{e.lap}</td>
                <td>{fmtLapTime(e.lastLapTime)}</td>
                <td>{fmtLapTime(e.bestLapTime)}</td>
                <td>{fmtGap(e.gapToLeader)}</td>
                <td>
                  {e.currentPitRoadSec != null
                    ? `● ${fmtSec(e.currentPitRoadSec)}`
                    : e.lastPitRoadSec != null
                    ? <span className="muted">({fmtSec(e.lastPitRoadSec)})</span>
                    : ''}
                </td>
                <td>{e.incidents}</td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </section>
  )
}
