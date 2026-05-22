export function windDeg(rad: number): number {
  return ((rad % (2 * Math.PI)) * 180 / Math.PI + 360) % 360
}

export function windCardinal(rad: number): string {
  const dirs = ['N', 'NE', 'E', 'SE', 'S', 'SW', 'W', 'NW']
  return dirs[Math.round(windDeg(rad) / 45) % 8]
}

// ─── Compass geometry (viewBox -20 -20 40 40) ─────────────────────────────────

const R = 17  // ring radius

const TICKS = Array.from({ length: 8 }, (_, i) => {
  const a = i * 45 * Math.PI / 180
  const sx = Math.sin(a)
  const cy = -Math.cos(a)
  const isCard = i % 2 === 0
  const innerR = isCard ? R - 4 : R - 2.5
  return { x1: sx * R, y1: cy * R, x2: sx * innerR, y2: cy * innerR, isCard }
})

const LABEL_R = 12.5
const LABELS = [
  { d: 'N', x: 0,        y: -LABEL_R, fill: '#e8e8e8', size: 5.5, weight: 700 },
  { d: 'E', x: LABEL_R,  y: 0,        fill: '#666',    size: 4.5, weight: 500 },
  { d: 'S', x: 0,        y: LABEL_R,  fill: '#666',    size: 4.5, weight: 500 },
  { d: 'W', x: -LABEL_R, y: 0,        fill: '#666',    size: 4.5, weight: 500 },
]

// ─── Component ────────────────────────────────────────────────────────────────

interface Props {
  windRad: number | null
  size?: number
  title?: string
}

// Compass overlaid on TrackMap: N always at top (absolute/North-up orientation).
// The needle tip (amber) points toward the direction FROM which wind originates.
export function WindCompass({ windRad, size = 48, title }: Props) {
  if (windRad == null) return null
  const deg = windDeg(windRad)

  return (
    <svg
      width={size} height={size}
      viewBox="-20 -20 40 40"
      style={{ display: 'block', overflow: 'visible' }}
    >
      {title && <title>{title}</title>}

      {/* Background disc */}
      <circle r="19.5" fill="rgba(0,0,0,0.68)" />

      {/* Ring */}
      <circle r={R} fill="none" stroke="#2c2c2c" strokeWidth="1.5" />

      {/* 8 tick marks */}
      {TICKS.map((t, i) => (
        <line key={i}
          x1={t.x1} y1={t.y1} x2={t.x2} y2={t.y2}
          stroke={t.isCard ? '#505050' : '#383838'}
          strokeWidth={t.isCard ? 1.5 : 1}
        />
      ))}

      {/* Cardinal labels — N always at top, compass is North-up */}
      {LABELS.map(l => (
        <text key={l.d}
          x={l.x} y={l.y}
          fill={l.fill}
          fontSize={l.size}
          fontWeight={l.weight}
          textAnchor="middle"
          dominantBaseline="middle"
          style={{ fontFamily: 'system-ui, sans-serif' }}
        >{l.d}</text>
      ))}

      {/* Wind needle — rotated to absolute wind direction (0° = from North) */}
      <g
        transform={`rotate(${deg})`}
        style={{ transition: 'transform 300ms ease-out' }}
      >
        {/* Amber tip: points toward the direction FROM which wind comes */}
        <path d="M 0 -10.5 L 2.8 1 L 0 -1.5 L -2.8 1 Z" fill="#f59e0b" />
        {/* Grey tail */}
        <path d="M 0 7.5 L 1.8 1 L 0 2 L -1.8 1 Z" fill="#484848" />
        {/* Center pivot */}
        <circle r="1.8" fill="#111" stroke="#505050" strokeWidth="0.8" />
      </g>
    </svg>
  )
}
