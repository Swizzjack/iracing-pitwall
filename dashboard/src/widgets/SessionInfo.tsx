import type { SessionInfoYaml } from '@shared/SessionInfoYaml'

interface Props {
  info: SessionInfoYaml | null
}

export function SessionInfo({ info }: Props) {
  if (!info) {
    return (
      <section className="card">
        <h2>Session</h2>
        <p className="muted">waiting for first frame…</p>
      </section>
    )
  }

  const { WeekendInfo: w, SessionInfo: s, DriverInfo: d } = info

  return (
    <section className="card">
      <h2>Session</h2>
      <div className="card-body">
        <dl className="kv">
          <dt>Track</dt>
          <dd>{w.TrackDisplayName || w.TrackName}</dd>
          <dt>Series</dt>
          <dd>{w.SeriesID}</dd>
          <dt>Subsession</dt>
          <dd className="mono">{String(w.SubSessionID)}</dd>
          <dt>Sessions</dt>
          <dd>{s.Sessions.length}</dd>
          <dt>Drivers</dt>
          <dd>{d.Drivers.length}</dd>
          <dt>Player CarIdx</dt>
          <dd>{d.DriverCarIdx}</dd>
        </dl>
        <details className="muted">
          <summary>Sessions</summary>
          <ul>
            {s.Sessions.map((sess) => (
              <li key={sess.SessionNum}>
                #{sess.SessionNum} {sess.SessionType} — {sess.SessionName}
                {sess.ResultsPositions && ` (${sess.ResultsPositions.length} results)`}
              </li>
            ))}
          </ul>
        </details>
      </div>
    </section>
  )
}
