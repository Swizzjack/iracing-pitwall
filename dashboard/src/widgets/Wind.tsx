import { useRef, useState } from 'react'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import type { SessionInfoYaml } from '@shared/SessionInfoYaml'
import { SettingsDrawer } from '../components/SettingsDrawer'
import {
  WindSettings,
  SPEED_UNIT_DEFAULT, MARKER_STYLE_DEFAULT,
  type SpeedUnit, type MarkerStyle,
} from './WindSettings'

// ─── Helpers ──────────────────────────────────────────────────────────────────

function fmtSpeedNum(ms: number, unit: SpeedUnit): string {
  if (unit === 'kmh') return (ms * 3.6).toFixed(1)
  if (unit === 'mph') return (ms * 2.23694).toFixed(1)
  return ms.toFixed(1)
}

function fmtSpeedUnit(unit: SpeedUnit): string {
  if (unit === 'kmh') return 'km/h'
  if (unit === 'mph') return 'mph'
  return 'm/s'
}

// Unwrap angle to prevent 360→0 transition jumps
function unwrapAngle(prev: number, next: number): number {
  const delta = ((next - prev) % 360 + 540) % 360 - 180
  return prev + delta
}

// Orange at 0° (headwind), blue at 180° (tailwind)
function windColor(relWindDeg: number): string {
  const norm = ((relWindDeg % 360) + 360) % 360
  const t = (Math.cos(norm * Math.PI / 180) + 1) / 2
  const r = Math.round(255 * t + 91 * (1 - t))
  const g = Math.round(107 * t + 201 * (1 - t))
  const b = Math.round(53 * t + 255 * (1 - t))
  return `rgb(${r},${g},${b})`
}

// ─── Compass data ─────────────────────────────────────────────────────────────

const RING_R = 44

// 8 tick marks: 4 cardinal (N/E/S/W) + 4 intercardinal
const COMPASS_TICKS = Array.from({ length: 8 }, (_, i) => {
  const deg = i * 45
  const rad = deg * (Math.PI / 180)
  const sin = Math.sin(rad)
  const cos = -Math.cos(rad)
  const isCardinal = i % 2 === 0
  const innerR = isCardinal ? RING_R - 5 : RING_R - 3
  return {
    x1: sin * RING_R,   y1: cos * RING_R,
    x2: sin * innerR,   y2: cos * innerR,
    stroke: '#5a5a5a',
    strokeWidth: isCardinal ? 2 : 1,
  }
})

const LABEL_R = 55
const COMPASS_LABELS = [
  { label: 'N', deg: 0,   color: '#E8E8E8', fontSize: 9,   fontWeight: 600 },
  { label: 'E', deg: 90,  color: '#9a9a9a', fontSize: 8,   fontWeight: 500 },
  { label: 'S', deg: 180, color: '#9a9a9a', fontSize: 8,   fontWeight: 500 },
  { label: 'W', deg: 270, color: '#9a9a9a', fontSize: 8,   fontWeight: 500 },
].map(l => ({
  ...l,
  x: Math.sin(l.deg * Math.PI / 180) * LABEL_R,
  y: -Math.cos(l.deg * Math.PI / 180) * LABEL_R,
}))

// ─── Wind marker shapes ───────────────────────────────────────────────────────

// Arrow with shaft pointing toward center (tip at y=-35, shaft up to y=-56).
function ArrowMarker({ color }: { color: string }) {
  return (
    <path
      d="M 0 -35 L -9 -46 L -3.5 -46 L -3.5 -57 L 3.5 -57 L 3.5 -46 L 9 -46 Z"
      fill={color}
    />
  )
}

// Three chevrons pointing toward center; outer pulses first → inner last,
// suggesting wind movement inward (toward the car).
function ChevronMarker({ color }: { color: string }) {
  // Each chevron: wide V-shape pointing down (toward center), filled as thin band.
  // outer: y -55..-49, middle: y -48..-42, inner: y -41..-35
  return (
    <g fill={color}>
      {/* outer */}
      <path
        className="wind-chevron-outer"
        d="M -10 -55 L 0 -49 L 10 -55 L 8 -57 L 0 -51 L -8 -57 Z"
      />
      {/* middle */}
      <path
        className="wind-chevron-mid"
        d="M -10 -48 L 0 -42 L 10 -48 L 8 -50 L 0 -44 L -8 -50 Z"
      />
      {/* inner */}
      <path
        className="wind-chevron-inner"
        d="M -10 -41 L 0 -35 L 10 -41 L 8 -43 L 0 -37 L -8 -43 Z"
      />
    </g>
  )
}

// ─── SVG gauge ────────────────────────────────────────────────────────────────

interface GaugeProps {
  isCalm: boolean
  relWindDeg: number
  carHeadingDeg: number
  markerColor: string
  markerStyle: MarkerStyle
  fontScale: number
  carImage: string
}

function WindGauge({ isCalm, relWindDeg, carHeadingDeg, markerColor, markerStyle, fontScale, carImage }: GaugeProps) {
  return (
    <svg
      viewBox="-57 -57 114 114"
      width="100%"
      height="100%"
      style={{ display: 'block', overflow: 'visible' }}
      aria-label="Wind direction compass"
    >
      {/* ── Ring (static outer circle) ────────────────────────────────── */}
      <circle r={RING_R} fill="none" stroke="#1a1a1a" strokeWidth="1.5" />

      {/* ── Compass rose (ticks + labels) rotate opposite to car heading  */}
      {/* so the label at the top always shows the car's current heading   */}
      <g
        transform={`rotate(${-carHeadingDeg})`}
        style={{ transition: 'transform 300ms ease-out' }}
      >
        {COMPASS_TICKS.map((t, i) => (
          <line key={i}
            x1={t.x1} y1={t.y1} x2={t.x2} y2={t.y2}
            stroke={t.stroke} strokeWidth={t.strokeWidth}
          />
        ))}
        {COMPASS_LABELS.map(l => (
          <text key={l.label}
            x={l.x} y={l.y}
            fill={l.color}
            fontSize={l.fontSize * fontScale}
            fontWeight={l.fontWeight}
            textAnchor="middle"
            dominantBaseline="middle"
            style={{ fontFamily: "'Teko', sans-serif" }}
          >{l.label}</text>
        ))}
      </g>

      {/* ── Car image, nose = top = driving direction ─────────────────── */}
      <image href={carImage} x="-34" y="-34" width="68" height="68" preserveAspectRatio="xMidYMid meet" />

      {/* ── Wind marker: rotated by relWindDeg (car-relative)            */}
      {!isCalm && (
        <g
          transform={`rotate(${relWindDeg})`}
          style={{ transition: 'transform 300ms ease-out' }}
        >
          {markerStyle === 'arrow'
            ? <ArrowMarker color={markerColor} />
            : <ChevronMarker color={markerColor} />
          }
        </g>
      )}
    </svg>
  )
}

// ─── localStorage ─────────────────────────────────────────────────────────────

const LS_UNITS        = 'iracing-wind-units-v1'
const LS_FONTSCALE    = 'iracing-wind-fontscale-v1'
const LS_MARKER_STYLE = 'iracing-wind-markerstyle-v1'

function loadSettings() {
  const load = (key: string, fallback: string) => {
    try { return localStorage.getItem(key) ?? fallback } catch { return fallback }
  }
  const loadNum = (key: string, fallback: number) => {
    try {
      const v = parseFloat(localStorage.getItem(key) ?? '')
      return isNaN(v) ? fallback : v
    } catch { return fallback }
  }
  return {
    speedUnit:   load(LS_UNITS, SPEED_UNIT_DEFAULT) as SpeedUnit,
    markerStyle: load(LS_MARKER_STYLE, MARKER_STYLE_DEFAULT) as MarkerStyle,
    fontScale:   loadNum(LS_FONTSCALE, 1),
  }
}

function save(key: string, value: string) {
  try { localStorage.setItem(key, value) } catch { /* ignore */ }
}

// ─── Main widget ──────────────────────────────────────────────────────────────

interface Props {
  snap: TelemetrySnapshot | null
  info?: SessionInfoYaml | null
}

export function Wind({ snap, info }: Props) {
  const [showSettings,  setShowSettings]  = useState(false)
  const [speedUnit,     setSpeedUnit]     = useState<SpeedUnit>(() => loadSettings().speedUnit)
  const [markerStyle,   setMarkerStyle]   = useState<MarkerStyle>(() => loadSettings().markerStyle)
  const [fontScale,     setFontScale]     = useState(() => loadSettings().fontScale)

  const prevWindDirRef    = useRef(0)
  const prevCarHeadingRef = useRef(0)

  function handleSpeedUnit(u: SpeedUnit)   { setSpeedUnit(u);     save(LS_UNITS, u) }
  function handleMarkerStyle(s: MarkerStyle) { setMarkerStyle(s); save(LS_MARKER_STYLE, s) }
  function handleFontScale(v: number)      { setFontScale(v);     save(LS_FONTSCALE, String(v)) }

  function handleResetAll() {
    setSpeedUnit(SPEED_UNIT_DEFAULT);     save(LS_UNITS, SPEED_UNIT_DEFAULT)
    setMarkerStyle(MARKER_STYLE_DEFAULT); save(LS_MARKER_STYLE, MARKER_STYLE_DEFAULT)
    setFontScale(1);                      save(LS_FONTSCALE, '1')
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

  const playerDriver = info?.DriverInfo?.Drivers.find(d => d.CarIdx === snap.playerCarIdx) ?? null
  const carPath = playerDriver?.CarPath ?? ''
  const carImage = /dallara|formula|skipbarber|usf|indy/i.test(carPath) ? '/carf.png' : '/car.png'

  const { windVel, windDir, yawNorth, yaw } = snap
  const carHeadingRad = yawNorth ?? yaw
  const isCalm = windVel == null || windDir == null || windVel < 0.5

  const rawCarHeadingDeg = ((carHeadingRad * 180 / Math.PI) % 360 + 360) % 360
  const carHeadingDeg = unwrapAngle(prevCarHeadingRef.current, rawCarHeadingDeg)
  prevCarHeadingRef.current = carHeadingDeg

  let relWindDeg = prevWindDirRef.current
  if (!isCalm) {
    const windDirDeg = ((windDir! * 180 / Math.PI) % 360 + 360) % 360
    const rawRelWind = ((windDirDeg - rawCarHeadingDeg) % 360 + 360) % 360
    relWindDeg = unwrapAngle(prevWindDirRef.current, rawRelWind)
    prevWindDirRef.current = relWindDeg
  }

  const markerColor = isCalm ? '#5BC9FF' : windColor(relWindDeg)

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
          markerStyle={markerStyle}
          fontScale={fontScale}
          onSpeedUnit={handleSpeedUnit}
          onMarkerStyle={handleMarkerStyle}
          onFontScale={handleFontScale}
          onResetAll={handleResetAll}
        />
      </SettingsDrawer>

      <div className="card-body" style={{ padding: '0 2px 6px', display: 'flex', flexDirection: 'column', gap: 4 }}>
        <div style={{ flex: '1 1 0', minHeight: 0, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <div style={{ width: '100%', maxWidth: '100%', maxHeight: '100%', aspectRatio: '1 / 1' }}>
            <WindGauge
              isCalm={isCalm}
              relWindDeg={relWindDeg}
              carHeadingDeg={carHeadingDeg}
              markerColor={markerColor}
              markerStyle={markerStyle}
              fontScale={fontScale}
              carImage={carImage}
            />
          </div>
        </div>

        {/* Speed display below compass */}
        <div style={{ flexShrink: 0, textAlign: 'right', lineHeight: 1, paddingRight: 4 }}>
          {isCalm ? (
            <span style={{
              color: '#5BC9FF',
              fontSize: `${14 * fontScale}px`,
              fontWeight: 400,
              opacity: 0.5,
              fontFamily: 'var(--font-mono)',
            }}>calm</span>
          ) : (
            <>
              <span style={{
                color: markerColor,
                fontSize: `${28 * fontScale}px`,
                fontWeight: 600,
                fontFamily: 'var(--font-mono)',
              }}>{fmtSpeedNum(windVel!, speedUnit)}</span>
              <span style={{
                color: markerColor,
                fontSize: `${13 * fontScale}px`,
                fontWeight: 400,
                marginLeft: 4,
                fontFamily: 'var(--font-mono)',
              }}>{fmtSpeedUnit(speedUnit)}</span>
            </>
          )}
        </div>
      </div>
    </section>
  )
}
