import { useRef, useState } from 'react'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import { windCardinal } from '../components/WindCompass'
import { SettingsDrawer } from '../components/SettingsDrawer'
import { WindSettings, SPEED_UNIT_DEFAULT, type SpeedUnit } from './WindSettings'

// ─── Helpers ──────────────────────────────────────────────────────────────────

function fmtSpeed(ms: number, unit: SpeedUnit): string {
  if (unit === 'kmh') return `${(ms * 3.6).toFixed(1)} km/h`
  if (unit === 'mph') return `${(ms * 2.23694).toFixed(1)} mph`
  return `${ms.toFixed(1)} m/s`
}

function windColor(ms: number): string {
  if (ms < 1)  return '#3b82f6'
  if (ms < 3)  return '#60a5fa'
  if (ms < 5)  return '#facc15'
  if (ms < 8)  return '#fb923c'
  return '#ef4444'
}

function unwrapAngle(prev: number, next: number): number {
  const delta = ((next - prev) % 360 + 540) % 360 - 180
  return prev + delta
}

// ─── Compass ring ─────────────────────────────────────────────────────────────

const COMPASS_TICKS = Array.from({ length: 8 }, (_, i) => {
  const angle = i * 45
  const rad = (angle - 90) * (Math.PI / 180)
  const isCardinal = i % 2 === 0
  return {
    angle,
    x1: Math.cos(rad) * 44, y1: Math.sin(rad) * 44,
    x2: Math.cos(rad) * (isCardinal ? 40 : 42), y2: Math.sin(rad) * (isCardinal ? 40 : 42),
    isCardinal,
  }
})

const COMPASS_LABELS = [
  { label: 'N', x: 0, y: -39, color: '#f97316' },
  { label: 'E', x: 39, y: 0, color: '#333' },
  { label: 'S', x: 0, y: 39, color: '#333' },
  { label: 'W', x: -39, y: 0, color: '#333' },
]

// ─── SVG gauge ────────────────────────────────────────────────────────────────

interface GaugeProps {
  windVel: number | null
  windDir: number | null
  yaw: number
  showCompass: boolean
  showStreamlines: boolean
  prevRelRef: React.MutableRefObject<number>
  prevYawRef: React.MutableRefObject<number>
}

function WindGauge({ windVel, windDir, yaw, showCompass, showStreamlines, prevRelRef, prevYawRef }: GaugeProps) {
  const isCalm = windVel == null || windDir == null || windVel < 0.3

  // unwrap angles for smooth CSS transitions (no wrap-around flip)
  const rawYawDeg = yaw * (180 / Math.PI)
  const yawDeg = unwrapAngle(prevYawRef.current, rawYawDeg)
  prevYawRef.current = yawDeg

  let relDeg = prevRelRef.current
  let color = '#3b82f6'
  let head = 0
  let cross = 0

  if (!isCalm) {
    const relRad = windDir! - yaw
    const rawRelDeg = relRad * (180 / Math.PI)
    relDeg = unwrapAngle(prevRelRef.current, rawRelDeg)
    prevRelRef.current = relDeg
    color = windColor(windVel!)
    head  = windVel! * Math.cos(relRad)
    cross = windVel! * Math.sin(relRad)
  }

  const rotStyle = (deg: number): React.CSSProperties => ({
    transform: `rotate(${deg}deg)`,
    transformOrigin: '50% 50%',
    transformBox: 'view-box',
    transition: 'transform 150ms ease-out',
  })

  const arrowStyle: React.CSSProperties = {
    ...rotStyle(relDeg),
    filter: !isCalm && windVel! > 4 ? `drop-shadow(0 0 2px ${color})` : undefined,
  }

  return (
    <svg
      viewBox="-50 -50 100 100"
      width="100%"
      height="100%"
      style={{ display: 'block', overflow: 'hidden' }}
      aria-label="Wind direction relative to car"
    >
      <defs>
        {/* clip everything to the backdrop circle so the arrow can never leave */}
        <clipPath id="wg-clip">
          <circle r="46" />
        </clipPath>
      </defs>

      {/* ── Backdrop ─────────────────────────────────────────────────── */}
      <circle r="47" fill="#0d0d0d" />

      <g clipPath="url(#wg-clip)">

        {/* ── Outer compass ring (rotates with absolute yaw) ──────────── */}
        {showCompass && (
          <g style={rotStyle(yawDeg)}>
            <circle r="44" fill="none" stroke="#1e1e1e" strokeWidth="1" />
            {COMPASS_TICKS.map((t, i) => (
              <line
                key={i}
                x1={t.x1} y1={t.y1}
                x2={t.x2} y2={t.y2}
                stroke={t.isCardinal ? '#303030' : '#222'}
                strokeWidth={t.isCardinal ? 1.5 : 1}
              />
            ))}
            {COMPASS_LABELS.map(l => (
              <text
                key={l.label}
                x={l.x} y={l.y}
                fill={l.color}
                fontSize="5"
                textAnchor="middle"
                dominantBaseline="middle"
                style={{ fontFamily: "'Teko', sans-serif", fontWeight: 500 }}
              >{l.label}</text>
            ))}
          </g>
        )}

        {/* ── Inner relative ring (static) ────────────────────────────── */}
        <circle r="32" fill="none" stroke="#1a1a1a" strokeWidth="0.5" strokeDasharray="2 4" />

        {/* relative labels HEAD / TAIL / L / R */}
        <text x="0" y="-36" fill="#2a2a2a" fontSize="3.5" textAnchor="middle" dominantBaseline="middle"
              style={{ fontFamily: "'Roboto Condensed', sans-serif", textTransform: 'uppercase', letterSpacing: '0.1em' }}>HEAD</text>
        <text x="0" y="36" fill="#2a2a2a" fontSize="3.5" textAnchor="middle" dominantBaseline="middle"
              style={{ fontFamily: "'Roboto Condensed', sans-serif", textTransform: 'uppercase', letterSpacing: '0.1em' }}>TAIL</text>
        <text x="-36" y="0" fill="#2a2a2a" fontSize="3.5" textAnchor="middle" dominantBaseline="middle"
              style={{ fontFamily: "'Roboto Condensed', sans-serif" }}>L</text>
        <text x="36" y="0" fill="#2a2a2a" fontSize="3.5" textAnchor="middle" dominantBaseline="middle"
              style={{ fontFamily: "'Roboto Condensed', sans-serif" }}>R</text>

        {/* ── Wind arrow (rotates with relative wind direction) ──────── */}
        {!isCalm && (
          <g style={arrowStyle}>
            {/* streamlines */}
            {showStreamlines && (
              <>
                <line
                  className="wind-streamline"
                  x1="-7" y1="-42" x2="-7" y2="-38"
                  stroke={color} strokeWidth="1" opacity="0.3"
                  style={{ animationDelay: '0s' }}
                />
                <line
                  className="wind-streamline"
                  x1="7" y1="-42" x2="7" y2="-38"
                  stroke={color} strokeWidth="1" opacity="0.3"
                  style={{ animationDelay: '0.65s' }}
                />
                <line
                  className="wind-streamline"
                  x1="0" y1="-42" x2="0" y2="-38"
                  stroke={color} strokeWidth="1" opacity="0.15"
                  style={{ animationDelay: '0.33s' }}
                />
              </>
            )}
            {/* shaft */}
            <line
              x1="0" y1="-43" x2="0" y2="-31"
              stroke={color} strokeWidth="2.5" strokeLinecap="round"
            />
            {/* arrowhead (pointing toward car = +y direction) */}
            <polygon points="0,-29 -5,-38 5,-38" fill={color} />
            {/* feather chevron at tail */}
            <polyline
              points="-4,-39 0,-43 4,-39"
              fill="none" stroke={color} strokeWidth="1.5" strokeLinejoin="round" strokeLinecap="round"
            />
          </g>
        )}

        {/* ── Car silhouette (static, pointing up = forward) ──────────── */}
        {/* body */}
        <rect x="-8" y="-22" width="16" height="43" rx="7" fill="#1c1c1c" stroke="#facc15" strokeWidth="1.5" />
        {/* front splitter */}
        <rect x="-12" y="-28" width="24" height="6" rx="3" fill="#1c1c1c" stroke="#facc15" strokeWidth="1.5" />
        {/* rear wing */}
        <rect x="-13" y="21" width="26" height="4" rx="2" fill="#1c1c1c" stroke="#facc15" strokeWidth="1.5" />
        {/* cockpit */}
        <ellipse cx="0" cy="-5" rx="4" ry="6" fill="#0a0a0a" stroke="#facc1545" strokeWidth="1" />

        {/* ── CALM overlay ─────────────────────────────────────────────── */}
        {isCalm && (
          <text
            x="0" y="0"
            textAnchor="middle" dominantBaseline="middle"
            fill="#333" fontSize="9"
            style={{ fontFamily: "'Teko', sans-serif", letterSpacing: '0.12em' }}
          >CALM</text>
        )}

      </g>{/* end clipPath group */}

      {/* Return values used outside this SVG via data-* hack: skip. Computed above. */}
      <g data-head={head} data-cross={cross} style={{ display: 'none' }} />
    </svg>
  )
}

// ─── Readout rows ─────────────────────────────────────────────────────────────

function ReadoutRow({ label, value, indicator, color }: { label: string; value: string; indicator: string; color?: string }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '4px 0', borderBottom: '1px solid #1a1a1a' }}>
      <span style={{
        color: '#3a3a3a', fontSize: 'calc(10px * var(--widget-font-scale, 1))',
        textTransform: 'uppercase', letterSpacing: '0.08em',
        flex: '0 0 58px', fontFamily: 'var(--font-body)',
      }}>{label}</span>
      <span style={{
        color: color ?? '#bbb', fontFamily: 'var(--font-mono)',
        fontSize: 'calc(12px * var(--widget-font-scale, 1))', flex: 1,
      }}>{value}</span>
      <span style={{
        color: '#444', fontSize: 'calc(11px * var(--widget-font-scale, 1))',
        flex: '0 0 auto', fontFamily: 'var(--font-body)',
      }}>{indicator}</span>
    </div>
  )
}

// ─── localStorage ─────────────────────────────────────────────────────────────

const LS_UNITS      = 'iracing-wind-units-v1'
const LS_COMPASS    = 'iracing-wind-compass-v1'
const LS_STREAMS    = 'iracing-wind-streams-v1'
const LS_FONTSCALE  = 'iracing-wind-fontscale-v1'

function loadSettings() {
  const load = (key: string, fallback: string) => {
    try { return localStorage.getItem(key) ?? fallback } catch { return fallback }
  }
  const loadBool = (key: string, fallback: boolean) => {
    try {
      const v = localStorage.getItem(key)
      return v === null ? fallback : v === 'true'
    } catch { return fallback }
  }
  const loadNum = (key: string, fallback: number) => {
    try {
      const v = parseFloat(localStorage.getItem(key) ?? '')
      return isNaN(v) ? fallback : v
    } catch { return fallback }
  }
  return {
    speedUnit:       load(LS_UNITS, SPEED_UNIT_DEFAULT) as SpeedUnit,
    showCompass:     loadBool(LS_COMPASS, true),
    showStreamlines: loadBool(LS_STREAMS, true),
    fontScale:       loadNum(LS_FONTSCALE, 1),
  }
}

function save(key: string, value: string) {
  try { localStorage.setItem(key, value) } catch { /* ignore */ }
}

// ─── Main widget ──────────────────────────────────────────────────────────────

interface Props {
  snap: TelemetrySnapshot | null
}

export function Wind({ snap }: Props) {
  const [showSettings, setShowSettings] = useState(false)

  const [speedUnit, setSpeedUnit]           = useState<SpeedUnit>(() => loadSettings().speedUnit)
  const [showCompass, setShowCompass]       = useState(() => loadSettings().showCompass)
  const [showStreamlines, setShowStreamlines] = useState(() => loadSettings().showStreamlines)
  const [fontScale, setFontScale]           = useState(() => loadSettings().fontScale)

  const prevRelRef = useRef(0)
  const prevYawRef = useRef(0)

  function handleSpeedUnit(u: SpeedUnit)   { setSpeedUnit(u);      save(LS_UNITS, u) }
  function handleCompass(v: boolean)        { setShowCompass(v);    save(LS_COMPASS, String(v)) }
  function handleStreamlines(v: boolean)    { setShowStreamlines(v); save(LS_STREAMS, String(v)) }
  function handleFontScale(v: number)       { setFontScale(v);      save(LS_FONTSCALE, String(v)) }

  function handleResetAll() {
    setSpeedUnit(SPEED_UNIT_DEFAULT); save(LS_UNITS, SPEED_UNIT_DEFAULT)
    setShowCompass(true);             save(LS_COMPASS, 'true')
    setShowStreamlines(true);         save(LS_STREAMS, 'true')
    setFontScale(1);                  save(LS_FONTSCALE, '1')
  }

  if (!snap) {
    return (
      <section className="card">
        <h2>Wind</h2>
        <div className="card-body">
          <p className="muted">waiting for first frame…</p>
        </div>
      </section>
    )
  }

  const { windVel, windDir, yaw } = snap
  const isCalm = windVel == null || windDir == null || windVel < 0.3

  // compute decomposition for readouts
  const relRad   = (!isCalm) ? windDir! - yaw : 0
  const head     = (!isCalm) ? windVel! * Math.cos(relRad) : 0
  const cross    = (!isCalm) ? windVel! * Math.sin(relRad) : 0

  const headLabel     = head >= 0 ? 'Headwind' : 'Tailwind'
  const headIndicator = head >= 0 ? '↓' : '↑'
  const headColor     = head >= 0 ? '#f97316' : '#60a5fa'
  const crossLabel    = Math.abs(cross) < 0.1 ? 'Cross' : cross > 0 ? 'Cross ←' : 'Cross →'
  const crossIndicator = Math.abs(cross) < 0.1 ? '—' : cross > 0 ? 'from L' : 'from R'
  const cardinal      = (!isCalm) ? `from ${windCardinal(windDir!)}` : ''

  return (
    <section className="card" style={{ '--widget-font-scale-local': fontScale } as React.CSSProperties}>
      <h2 style={{ display: 'flex', alignItems: 'center' }}>
        Wind
        <button
          className={`header-btn${showSettings ? ' header-btn-active' : ''}`}
          style={{ marginLeft: 'auto' }}
          onClick={() => setShowSettings(s => !s)}
        >⚙</button>
      </h2>

      <SettingsDrawer open={showSettings} onClose={() => setShowSettings(false)} title="Wind Settings">
        <WindSettings
          speedUnit={speedUnit}
          showCompass={showCompass}
          showStreamlines={showStreamlines}
          fontScale={fontScale}
          onSpeedUnit={handleSpeedUnit}
          onShowCompass={handleCompass}
          onShowStreamlines={handleStreamlines}
          onFontScale={handleFontScale}
          onResetAll={handleResetAll}
        />
      </SettingsDrawer>

      <div className="card-body" style={{ padding: '0 0 8px', display: 'flex', flexDirection: 'column', gap: 0 }}>
        {/* SVG gauge — fills available space */}
        <div style={{ flex: '1 1 0', minHeight: 0, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <div style={{ width: '100%', maxHeight: '100%', aspectRatio: '1 / 1' }}>
            <WindGauge
              windVel={windVel}
              windDir={windDir}
              yaw={yaw}
              showCompass={showCompass}
              showStreamlines={showStreamlines}
              prevRelRef={prevRelRef}
              prevYawRef={prevYawRef}
            />
          </div>
        </div>

        {/* Readout rows */}
        <div style={{ flex: '0 0 auto', padding: '4px 12px 0' }}>
          {isCalm ? (
            <div style={{ textAlign: 'center', color: '#333', fontSize: 'calc(11px * var(--widget-font-scale, 1))', padding: '6px 0', borderTop: '1px solid #1a1a1a' }}>
              No significant wind
            </div>
          ) : (
            <>
              <ReadoutRow
                label={headLabel}
                value={fmtSpeed(Math.abs(head), speedUnit)}
                indicator={headIndicator}
                color={headColor}
              />
              <ReadoutRow
                label={crossLabel}
                value={fmtSpeed(Math.abs(cross), speedUnit)}
                indicator={crossIndicator}
              />
              <ReadoutRow
                label="Total"
                value={fmtSpeed(windVel!, speedUnit)}
                indicator={cardinal}
                color="#ccc"
              />
            </>
          )}
        </div>
      </div>
    </section>
  )
}
