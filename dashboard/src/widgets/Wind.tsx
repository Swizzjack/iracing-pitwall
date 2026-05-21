import { useRef, useState } from 'react'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import { SettingsDrawer } from '../components/SettingsDrawer'
import { WindSettings, SPEED_UNIT_DEFAULT, type SpeedUnit } from './WindSettings'

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
  { label: 'N', deg: 0,   color: '#E8E8E8', fontSize: 7,   fontWeight: 600 },
  { label: 'E', deg: 90,  color: '#9a9a9a', fontSize: 6,   fontWeight: 500 },
  { label: 'S', deg: 180, color: '#9a9a9a', fontSize: 6,   fontWeight: 500 },
  { label: 'W', deg: 270, color: '#9a9a9a', fontSize: 6,   fontWeight: 500 },
].map(l => ({
  ...l,
  x: Math.sin(l.deg * Math.PI / 180) * LABEL_R,
  y: -Math.cos(l.deg * Math.PI / 180) * LABEL_R,
}))

// ─── SVG gauge ────────────────────────────────────────────────────────────────

interface GaugeProps {
  isCalm: boolean
  relWindDeg: number
  carHeadingDeg: number
  markerColor: string
}

function WindGauge({ isCalm, relWindDeg, carHeadingDeg, markerColor }: GaugeProps) {
  return (
    <svg
      viewBox="-62 -62 124 124"
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
            fontSize={l.fontSize}
            fontWeight={l.fontWeight}
            textAnchor="middle"
            dominantBaseline="middle"
            style={{ fontFamily: "'Teko', sans-serif" }}
          >{l.label}</text>
        ))}
      </g>

      {/* ── Car silhouette at 90%, nose = top = driving direction ─────── */}
      <g transform="scale(0.9)">
        <path d="M -3 -38 L 3 -38 L 11 -22 L 11 28 L -11 28 L -11 -22 Z" fill="#FFCC00" />
        <rect x="-15" y="-24" width="5" height="12" rx="1.5" fill="#484848" />
        <rect x="10"  y="-24" width="5" height="12" rx="1.5" fill="#484848" />
        <rect x="-15" y="10"  width="5" height="15" rx="1.5" fill="#484848" />
        <rect x="10"  y="10"  width="5" height="15" rx="1.5" fill="#484848" />
        <ellipse cx="0" cy="-2" rx="4" ry="6" fill="#1a1a1a" />
        <rect x="-18" y="30" width="36" height="5" rx="1.5" fill="#FFCC00" />
      </g>

      {/* ── Wind indicator: triangle, relative to car heading.           */}
      {/* Triangle at top (y=-53..-34) rotated by relWindDeg, so it shows */}
      {/* the compass direction FROM which wind hits the car.              */}
      {!isCalm && (
        <g
          transform={`rotate(${relWindDeg})`}
          style={{ transition: 'transform 300ms ease-out' }}
        >
          <path d="M -11 -53 L 11 -53 L 0 -34 Z" fill={markerColor} />
        </g>
      )}
    </svg>
  )
}

// ─── localStorage ─────────────────────────────────────────────────────────────

const LS_UNITS     = 'iracing-wind-units-v1'
const LS_FONTSCALE = 'iracing-wind-fontscale-v1'

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
    speedUnit: load(LS_UNITS, SPEED_UNIT_DEFAULT) as SpeedUnit,
    fontScale:  loadNum(LS_FONTSCALE, 1),
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
  const [speedUnit, setSpeedUnit] = useState<SpeedUnit>(() => loadSettings().speedUnit)
  const [fontScale, setFontScale]  = useState(() => loadSettings().fontScale)

  const prevWindDirRef    = useRef(0)
  const prevCarHeadingRef = useRef(0)

  function handleSpeedUnit(u: SpeedUnit) { setSpeedUnit(u); save(LS_UNITS, u) }
  function handleFontScale(v: number)    { setFontScale(v); save(LS_FONTSCALE, String(v)) }

  function handleResetAll() {
    setSpeedUnit(SPEED_UNIT_DEFAULT); save(LS_UNITS, SPEED_UNIT_DEFAULT)
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

  const { windVel, windDir, yawNorth, yaw } = snap
  const carHeadingRad = yawNorth ?? yaw
  const isCalm = windVel == null || windDir == null || windVel < 0.5

  // Compute angles here so markerColor is available for the speed display too
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
          fontScale={fontScale}
          onSpeedUnit={handleSpeedUnit}
          onFontScale={handleFontScale}
          onResetAll={handleResetAll}
        />
      </SettingsDrawer>

      <div className="card-body" style={{ padding: '0 4px 8px', display: 'flex', flexDirection: 'column', gap: 6 }}>
        <div style={{ flex: '1 1 0', minHeight: 0, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <div style={{ width: '100%', maxWidth: '100%', maxHeight: '100%', aspectRatio: '1 / 1' }}>
            <WindGauge
              isCalm={isCalm}
              relWindDeg={relWindDeg}
              carHeadingDeg={carHeadingDeg}
              markerColor={markerColor}
            />
          </div>
        </div>

        {/* Speed display below compass */}
        <div style={{ flexShrink: 0, textAlign: 'center', lineHeight: 1 }}>
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
