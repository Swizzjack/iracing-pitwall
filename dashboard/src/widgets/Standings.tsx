import type { StandingsSnapshot } from '@shared/StandingsSnapshot'
import { fmtLapTime } from '../format'

interface Props {
  snap: StandingsSnapshot | null
  playerCarIdx: number | null
}

const fmtGap = (g: number | null) => (g == null ? '—' : g === 0 ? '—' : `+${g.toFixed(3)}`)

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
          {snap.entries.map((e) => (
            <tr key={e.carIdx} className={e.carIdx === playerCarIdx ? 'me' : ''}>
              <td>{e.position}</td>
              <td>{e.classPosition}</td>
              <td>{e.carNumber}</td>
              <td className="name">{e.userName}</td>
              <td>{e.lap}</td>
              <td>{fmtLapTime(e.lastLapTime)}</td>
              <td>{fmtLapTime(e.bestLapTime)}</td>
              <td>{fmtGap(e.gapToLeader)}</td>
              <td>{e.onPitRoad ? '●' : ''}</td>
              <td>{e.incidents}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  )
}
