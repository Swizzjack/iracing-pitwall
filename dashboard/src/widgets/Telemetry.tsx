import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'

import { fmtLapTime } from '../format'

const fmt = (n: number, digits = 1) => n.toFixed(digits)
const kmh = (ms: number) => ms * 3.6
const gearLabel = (g: number) => (g === -1 ? 'R' : g === 0 ? 'N' : String(g))

interface Props {
  snap: TelemetrySnapshot | null
}

export function Telemetry({ snap }: Props) {
  if (!snap) {
    return (
      <section className="card">
        <h2>Telemetry</h2>
        <p className="muted">waiting for first frame…</p>
      </section>
    )
  }

  return (
    <section className="card">
      <h2>Telemetry</h2>
      <div className="card-body">
        <div className="big-row">
          <div className="big-stat">
            <div className="big-value">{fmt(kmh(snap.speedMs), 0)}</div>
            <div className="big-label">km/h</div>
          </div>
          <div className="big-stat">
            <div className="big-value">{gearLabel(snap.gear)}</div>
            <div className="big-label">gear</div>
          </div>
          <div className="big-stat">
            <div className="big-value">{fmt(snap.rpm, 0)}</div>
            <div className="big-label">rpm</div>
          </div>
        </div>

        <div className="bars">
          <Bar label="throttle" value={snap.throttle} color="#22c55e" />
          <Bar label="brake" value={snap.brake} color="#ef4444" />
        </div>

        <dl className="kv">
          <dt>lap</dt>
          <dd>{snap.lap}</dd>
          <dt>lap dist</dt>
          <dd>{fmt(snap.lapDistPct * 100, 1)} %</dd>
          <dt>cur lap</dt>
          <dd>{fmtLapTime(snap.lapCurrentTime)}</dd>
          <dt>last lap</dt>
          <dd>{fmtLapTime(snap.lapLastTime)}</dd>
          <dt>best lap</dt>
          <dd>{fmtLapTime(snap.lapBestTime)}</dd>
          <dt>fuel</dt>
          <dd>{fmt(snap.fuelLevel, 2)} L ({fmt(snap.fuelLevelPct * 100, 0)} %)</dd>
          <dt>fuel/h</dt>
          <dd>{fmt(snap.fuelUsePerHour, 2)} L/h</dd>
          <dt>position</dt>
          <dd>{snap.playerPosition} (cls {snap.playerClassPosition})</dd>
          <dt>track / air</dt>
          <dd>{fmt(snap.trackTemp, 1)} °C / {fmt(snap.airTemp, 1)} °C</dd>
          <dt>flags</dt>
          <dd className="mono">
            {snap.isOnTrack ? 'on-track ' : ''}
            {snap.isOnTrackCar ? 'in-car ' : ''}
            {snap.isInGarage ? 'garage ' : ''}
            {snap.onPitRoad ? 'pit-road' : ''}
          </dd>
        </dl>
      </div>
    </section>
  )
}

function Bar({ label, value, color }: { label: string; value: number; color: string }) {
  const pct = Math.max(0, Math.min(1, value)) * 100
  return (
    <div className="bar">
      <div className="bar-label">{label}</div>
      <div className="bar-track">
        <div className="bar-fill" style={{ width: `${pct}%`, background: color }} />
      </div>
      <div className="bar-value">{pct.toFixed(0)}</div>
    </div>
  )
}
