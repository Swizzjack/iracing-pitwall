import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import type { ResolvedField } from './Electronics'

interface Props {
  snap: TelemetrySnapshot
  fields: ResolvedField[]
  fontScale: number
}

const MGUK_MODES: Record<number, string> = {
  0: 'Off', 1: 'Low', 2: 'Medium', 3: 'High', 4: 'Hot Lap', 5: 'Time Attack',
}

function gearLabel(g: number) {
  if (g === -1) return 'R'
  if (g === 0) return 'N'
  return String(g)
}

// ─── SVG RPM Arc ─────────────────────────────────────────────────────────────

function describeArc(cx: number, cy: number, r: number, startDeg: number, endDeg: number) {
  const toRad = (d: number) => (d * Math.PI) / 180
  const sx = cx + r * Math.cos(toRad(startDeg))
  const sy = cy + r * Math.sin(toRad(startDeg))
  const ex = cx + r * Math.cos(toRad(endDeg))
  const ey = cy + r * Math.sin(toRad(endDeg))
  const sweep = endDeg > startDeg ? 0 : 1
  const large = Math.abs(endDeg - startDeg) > 180 ? 1 : 0
  return `M ${sx.toFixed(2)} ${sy.toFixed(2)} A ${r} ${r} 0 ${large} ${sweep} ${ex.toFixed(2)} ${ey.toFixed(2)}`
}

function RpmArc({ snap }: { snap: TelemetrySnapshot }) {
  const cx = 100
  const cy = 95
  const r = 72
  const startDeg = 215  // lower-left (in SVG coords)
  const endDeg = 325    // lower-right

  const maxRpm = snap.shiftLightLastRpm ?? Math.max(snap.rpm * 1.05, 8000)
  const rpmPct = Math.min(snap.rpm / maxRpm, 1)

  // Arc goes CCW from 215° to 325° (through top) = 250° span
  const totalSpan = 250
  const arcSpan = rpmPct * totalSpan
  // CCW from startDeg: progress endpoint angle = startDeg - arcSpan
  const progressEndDeg = startDeg - arcSpan

  const isNormal = snap.shiftLightShiftRpm == null || snap.rpm < snap.shiftLightShiftRpm
  const isBlink = snap.shiftLightBlinkRpm != null && snap.rpm >= snap.shiftLightBlinkRpm
  const arcColor = isBlink ? '#ef4444' : isNormal ? '#22c55e' : '#facc15'

  // Threshold markers
  const shiftPct = snap.shiftLightShiftRpm != null ? snap.shiftLightShiftRpm / maxRpm : null
  const blinkPct = snap.shiftLightBlinkRpm != null ? snap.shiftLightBlinkRpm / maxRpm : null

  function markerAt(pct: number) {
    const angleDeg = startDeg - pct * totalSpan
    const rad = (angleDeg * Math.PI) / 180
    const x1 = cx + (r - 5) * Math.cos(rad)
    const y1 = cy + (r - 5) * Math.sin(rad)
    const x2 = cx + (r + 8) * Math.cos(rad)
    const y2 = cy + (r + 8) * Math.sin(rad)
    return { x1, y1, x2, y2 }
  }

  return (
    <svg viewBox="0 0 200 130" style={{ width: '100%', overflow: 'visible' }}>
      {/* Background arc */}
      <path
        d={describeArc(cx, cy, r, startDeg, endDeg)}
        fill="none" stroke="#1a1a1a" strokeWidth={8} strokeLinecap="round"
      />
      {/* Progress arc (only draw if rpm > 0) */}
      {rpmPct > 0.002 && (
        <path
          d={describeArc(cx, cy, r, startDeg, progressEndDeg)}
          fill="none"
          stroke={arcColor}
          strokeWidth={8}
          strokeLinecap="round"
          style={{
            filter: isBlink ? `drop-shadow(0 0 4px ${arcColor})` : undefined,
            transition: isBlink ? undefined : 'stroke 0.1s',
          }}
        />
      )}
      {/* Threshold marker lines */}
      {shiftPct != null && (() => {
        const m = markerAt(shiftPct)
        return <line x1={m.x1} y1={m.y1} x2={m.x2} y2={m.y2} stroke="#facc1570" strokeWidth={1.5} />
      })()}
      {blinkPct != null && (() => {
        const m = markerAt(blinkPct)
        return <line x1={m.x1} y1={m.y1} x2={m.x2} y2={m.y2} stroke="#ef444470" strokeWidth={1.5} />
      })()}
    </svg>
  )
}

// ─── Side stat ────────────────────────────────────────────────────────────────

function Stat({ label, value, color = '#ccc', sub }: { label: string; value: string; color?: string; sub?: string }) {
  return (
    <div style={{ textAlign: 'center' }}>
      <div style={{ color: '#444', fontSize: 9, textTransform: 'uppercase', letterSpacing: '0.07em', marginBottom: 2 }}>{label}</div>
      <div style={{ color, fontFamily: 'var(--font-mono)', fontSize: 13, lineHeight: 1 }}>
        {value}
        {sub && <span style={{ color: '#555', fontSize: 9 }}> {sub}</span>}
      </div>
    </div>
  )
}

// ─── Bottom LED strip ─────────────────────────────────────────────────────────

function BottomStrip({ snap, fields }: { snap: TelemetrySnapshot; fields: ResolvedField[] }) {
  const has = (id: string) => fields.some(f => f.id === id)
  const items: { key: string; label: string; active: boolean; color: string }[] = []

  if (has('tc1') && snap.dcTractionControl != null) {
    const level = snap.dcTractionControl > 1.5 ? Math.round(snap.dcTractionControl) : Math.round(snap.dcTractionControl * 12)
    items.push({ key: 'tc1', label: `TC ${level}`, active: level > 0, color: '#22c55e' })
  }
  if (has('tc2') && snap.dcTractionControl2 != null) {
    const level = snap.dcTractionControl2 > 1.5 ? Math.round(snap.dcTractionControl2) : Math.round(snap.dcTractionControl2 * 12)
    items.push({ key: 'tc2', label: `TC2 ${level}`, active: level > 0, color: '#22c55e' })
  }
  if (has('abs') && snap.dcAbs != null) {
    const level = snap.dcAbs > 1.5 ? Math.round(snap.dcAbs) : Math.round(snap.dcAbs * 12)
    items.push({ key: 'abs', label: `ABS ${level}`, active: level > 0, color: '#3b82f6' })
  }
  if (has('bb')) {
    const bbVal = snap.brakeBias > 1 ? snap.brakeBias.toFixed(1) : (snap.brakeBias * 100).toFixed(1)
    items.push({ key: 'bb', label: `BB ${bbVal}%`, active: false, color: '#ccc' })
  }
  if (has('mgukMode') && snap.mgukDeployMode != null) {
    const mode = MGUK_MODES[snap.mgukDeployMode] ?? `M${snap.mgukDeployMode}`
    items.push({ key: 'mguk', label: `MGUK ${mode}`, active: snap.mgukDeployMode > 0, color: '#3b82f6' })
  }
  if (has('drs') && snap.drsStatus != null) {
    items.push({
      key: 'drs', label: 'DRS',
      active: snap.drsStatus >= 2,
      color: snap.drsStatus >= 2 ? '#22c55e' : snap.drsStatus >= 1 ? '#facc15' : '#555',
    })
  }
  if (has('p2p') && (snap.p2pCount != null || snap.p2pStatus != null)) {
    items.push({ key: 'p2p', label: `P2P${snap.p2pCount != null ? ` ×${snap.p2pCount}` : ''}`, active: snap.p2pStatus === true, color: '#3b82f6' })
  }

  // Engine warnings
  const warnings = snap.engineWarnings
  const warnBits = [
    { bit: 0x01, label: 'H₂O' }, { bit: 0x04, label: 'OIL-P' },
    { bit: 0x08, label: 'STALL' }, { bit: 0x10, label: 'PIT' },
    { bit: 0x20, label: 'REV' }, { bit: 0x40, label: 'OIL-T' },
  ]
  warnBits.forEach(({ bit, label }) => {
    if ((warnings & bit) !== 0) {
      items.push({ key: `w${bit}`, label, active: true, color: bit === 0x10 ? '#f97316' : '#ef4444' })
    }
  })

  if (items.length === 0) return null

  return (
    <div style={{
      display: 'flex', gap: 5, flexWrap: 'wrap', justifyContent: 'center',
      marginTop: 8, padding: '5px 0', borderTop: '1px solid #1a1a1a',
    }}>
      {items.map(({ key, label, active, color }) => (
        <span key={key} style={{
          fontSize: 10, fontFamily: 'var(--font-mono)', padding: '2px 6px', borderRadius: 3,
          background: active ? `${color}18` : '#0f0f0f',
          color: active ? color : '#333',
          border: `1px solid ${active ? `${color}35` : '#1a1a1a'}`,
          transition: 'all 0.15s',
          boxShadow: active ? `0 0 6px ${color}30` : undefined,
        }}>{label}</span>
      ))}
    </div>
  )
}

// ─── Main Export ─────────────────────────────────────────────────────────────

export function ElectronicsCluster({ snap, fields, fontScale }: Props) {
  const has = (id: string) => fields.some(f => f.id === id)

  const waterColor = snap.waterTemp > 105 ? '#ef4444' : snap.waterTemp > 95 ? '#facc15' : '#22c55e'
  const oilColor = snap.oilTemp > 130 ? '#ef4444' : snap.oilTemp > 115 ? '#facc15' : '#22c55e'
  const voltColor = snap.voltage < 12 ? '#ef4444' : snap.voltage < 13 ? '#facc15' : '#22c55e'
  const oilPressColor = snap.oilPress < 2.5 ? '#ef4444' : '#ccc'

  return (
    <div style={{ '--widget-font-scale': fontScale } as React.CSSProperties}>
      {/* Arc + center */}
      <div style={{ position: 'relative' }}>
        <RpmArc snap={snap} />

        {/* Center overlay */}
        <div style={{
          position: 'absolute',
          top: '20%', left: '50%', transform: 'translateX(-50%)',
          textAlign: 'center', pointerEvents: 'none',
        }}>
          {/* Gear */}
          <div style={{
            fontFamily: 'var(--font-mono)', fontWeight: 700,
            fontSize: `calc(${42 * fontScale}px)`, lineHeight: 1,
            color: snap.gear === -1 ? '#ef4444' : snap.gear === 0 ? '#facc15' : '#f0f0f0',
            textShadow: '0 0 30px rgba(249,115,22,0.2)',
          }}>
            {gearLabel(snap.gear)}
          </div>
          {/* RPM below gear */}
          <div style={{
            color: '#555', fontFamily: 'var(--font-mono)',
            fontSize: `calc(${10 * fontScale}px)`, marginTop: 2,
          }}>
            {snap.rpm.toFixed(0)} rpm
          </div>
        </div>

        {/* Left side stats */}
        <div style={{
          position: 'absolute', left: 4, top: '30%',
          display: 'flex', flexDirection: 'column', gap: 10,
        }}>
          {has('waterTemp') && (
            <Stat label="H₂O" value={`${snap.waterTemp.toFixed(0)}°`} color={waterColor} />
          )}
          {has('oilTemp') && (
            <Stat label="Oil°" value={`${snap.oilTemp.toFixed(0)}°`} color={oilColor} />
          )}
          {has('oilPress') && (
            <Stat label="Oil-P" value={snap.oilPress.toFixed(1)} color={oilPressColor} sub="bar" />
          )}
        </div>

        {/* Right side stats */}
        <div style={{
          position: 'absolute', right: 4, top: '30%',
          display: 'flex', flexDirection: 'column', gap: 10,
        }}>
          {has('voltage') && (
            <Stat label="VOLT" value={snap.voltage.toFixed(1)} color={voltColor} sub="V" />
          )}
          {has('speed') && (
            <Stat label="km/h" value={(snap.speedMs * 3.6).toFixed(0)} />
          )}
          {has('manifoldPress') && snap.manifoldPress != null && (
            <Stat label="Boost" value={snap.manifoldPress.toFixed(2)} color="#3b82f6" sub="bar" />
          )}
          {has('battery') && snap.energyBatteryPct != null && (() => {
            const pct = snap.energyBatteryPct
            const c = pct > 0.3 ? '#3b82f6' : pct > 0.15 ? '#facc15' : '#ef4444'
            return <Stat label="ERS" value={`${(pct * 100).toFixed(0)}%`} color={c} />
          })()}
        </div>
      </div>

      {/* Bottom LED strip */}
      <BottomStrip snap={snap} fields={fields} />
    </div>
  )
}
