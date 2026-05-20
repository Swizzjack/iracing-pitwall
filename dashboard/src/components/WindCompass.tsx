export function windDeg(rad: number): number {
  return ((rad % (2 * Math.PI)) * 180 / Math.PI + 360) % 360
}

export function windCardinal(rad: number): string {
  const dirs = ['N', 'NE', 'E', 'SE', 'S', 'SW', 'W', 'NW']
  return dirs[Math.round(windDeg(rad) / 45) % 8]
}

interface Props {
  windRad: number | null
  size?: number
  opacity?: number
  title?: string
}

const CARD_POS = [[0, -13], [13, 0], [0, 13], [-13, 0]] as const

export function WindCompass({ windRad, size = 40, opacity = 1, title }: Props) {
  if (windRad == null) return null
  const deg = windDeg(windRad)
  return (
    <svg width={size} height={size} viewBox="-20 -20 40 40"
      style={{ flexShrink: 0, overflow: 'visible', opacity }}>
      {title && <title>{title}</title>}
      <circle r="18" fill="none" stroke="#1f1f1f" strokeWidth="1.5" />
      {(['N', 'E', 'S', 'W'] as const).map((d, i) => (
        <text key={d} x={CARD_POS[i][0]} y={CARD_POS[i][1]}
          fill="#555" fontSize="5"
          textAnchor="middle" dominantBaseline="middle">{d}</text>
      ))}
      <g transform={`rotate(${deg})`}>
        <polygon points="0,-12 2.5,2 0,-1 -2.5,2" fill="#facc15" />
      </g>
    </svg>
  )
}
