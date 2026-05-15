import { useRef, useEffect, useState } from 'react'
import type { TrackMapSnapshot } from '@shared/TrackMapSnapshot'
import type { TrackShape } from '@shared/TrackShape'
import type { TrackCar } from '@shared/TrackCar'
import type { SessionInfoYaml } from '@shared/SessionInfoYaml'
import { SettingsDrawer } from '../components/SettingsDrawer'
import { TrackMapSettings } from './TrackMapSettings'

interface Props {
  snap: TrackMapSnapshot | null
  playerCarIdx: number | null
  info: SessionInfoYaml | null
}

// ─── Settings persistence ─────────────────────────────────────────────────────

const TRACK_W_KEY    = 'iracing-trackmap-track-width'
const CAR_R_KEY      = 'iracing-trackmap-car-size'
const SF_LEN_KEY     = 'iracing-trackmap-sf-length'
const SECTOR_SHOW_KEY = 'iracing-trackmap-sector-show'
const SECTOR_LEN_KEY  = 'iracing-trackmap-sector-length'
const FONT_SIZE_KEY   = 'iracing-trackmap-font-size'

function loadNum(key: string, def: number, min: number, max: number): number {
  try {
    const v = parseFloat(localStorage.getItem(key) ?? '')
    if (Number.isFinite(v) && v >= min && v <= max) return v
  } catch {}
  return def
}

interface DrawSettings {
  trackWidth: number
  carRadius: number
  sfLength: number
  sectorShow: boolean
  sectorLength: number
}

// ─── Color helpers ────────────────────────────────────────────────────────────

function classColor(hex: number | bigint | null): string {
  if (hex === null || hex === undefined) return '#888888'
  const n = typeof hex === 'bigint' ? Number(hex) : hex
  const c = Math.max(0, Math.min(0xffffff, n))
  return `#${c.toString(16).padStart(6, '0')}`
}

// ─── Interpolation helper ─────────────────────────────────────────────────────

function lerpPct(a: number, b: number, t: number): number {
  let d = b - a
  if (d >  0.5) d -= 1  // shortest path across S/F wrap
  if (d < -0.5) d += 1
  return ((a + d * t) % 1 + 1) % 1
}

// ─── Shape path cache ─────────────────────────────────────────────────────────

interface SectorMarker {
  x: number
  y: number
  nx: number
  ny: number
  num: number  // sector number label (S2, S3, …)
}

interface ShapeCache {
  key: string
  w: number
  h: number
  outerPath: Path2D
  innerPath: Path2D
  startX: number
  startY: number
  startNormalX: number
  startNormalY: number
  sectorMarkers: SectorMarker[]
  toScreen: (x: number, y: number) => [number, number]
  ptAtP: (p: number) => [number, number]
}

function buildShapeCache(shape: TrackShape, sectors: number[], w: number, h: number, key: string): ShapeCache {
  const PAD = 20
  const [xmin, ymin, xmax, ymax] = shape.bounds
  const rw = xmax - xmin
  const rh = ymax - ymin
  if (rw === 0 || rh === 0) {
    const noop: ShapeCache = {
      key, w, h,
      outerPath: new Path2D(),
      innerPath: new Path2D(),
      startX: w / 2,
      startY: h / 2,
      startNormalX: 0,
      startNormalY: 1,
      sectorMarkers: [],
      toScreen: () => [w / 2, h / 2],
      ptAtP: () => [w / 2, h / 2],
    }
    return noop
  }
  const s = Math.min((w - PAD * 2) / rw, (h - PAD * 2) / rh)
  const cx = w / 2 - ((xmin + xmax) / 2) * s
  const cy = h / 2 + ((ymin + ymax) / 2) * s  // y flipped (world +Y = up, canvas +Y = down)

  const toScreen = (x: number, y: number): [number, number] => [
    cx + x * s,
    cy - y * s,
  ]

  const pts = shape.points
  const n = pts.length

  // Binary search for interpolated screen position at lap dist pct p.
  const ptAtP = (p: number): [number, number] => {
    if (n === 0) return [w / 2, h / 2]
    const norm = ((p % 1) + 1) % 1
    let lo = 0
    let hi = n - 1
    while (lo < hi) {
      const mid = (lo + hi) >> 1
      if (pts[mid].p < norm) lo = mid + 1
      else hi = mid
    }
    const a = pts[(lo - 1 + n) % n]
    const b = pts[lo]
    const dp = b.p - a.p
    const t = dp > 1e-6 ? (norm - a.p) / dp : 0
    return toScreen(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
  }

  // Build outer path (thick, "asphalt").
  const outer = new Path2D()
  for (let i = 0; i < n; i++) {
    const [sx, sy] = toScreen(pts[i].x, pts[i].y)
    if (i === 0) outer.moveTo(sx, sy)
    else outer.lineTo(sx, sy)
  }
  outer.closePath()

  // Inner path — same route, thinner stroke for centre line.
  const inner = new Path2D()
  for (let i = 0; i < n; i++) {
    const [sx, sy] = toScreen(pts[i].x, pts[i].y)
    if (i === 0) inner.moveTo(sx, sy)
    else inner.lineTo(sx, sy)
  }
  inner.closePath()

  const [startX, startY] = toScreen(pts[0].x, pts[0].y)

  // Compute the unit normal at p=0 in screen space to draw the S/F line perpendicular
  // to the track tangent. We use the two neighbours of the first point.
  const [ax, ay] = toScreen(pts[(n - 1) % n].x, pts[(n - 1) % n].y)
  const [bx, by] = toScreen(pts[1 % n].x, pts[1 % n].y)
  const dx = bx - ax
  const dy = by - ay
  const len = Math.hypot(dx, dy) || 1
  const startNormalX = -dy / len
  const startNormalY =  dx / len

  // Build sector markers using the same tangent-normal technique.
  const EPS = 1 / RESAMPLE_N
  const sectorMarkers: SectorMarker[] = sectors.map((p, i) => {
    const [mx, my] = ptAtP(p)
    const [pax, pay] = ptAtP(p - EPS)
    const [pbx, pby] = ptAtP(p + EPS)
    const sdx = pbx - pax
    const sdy = pby - pay
    const slen = Math.hypot(sdx, sdy) || 1
    return { x: mx, y: my, nx: -sdy / slen, ny: sdx / slen, num: i + 2 }
  })

  return { key, w, h, outerPath: outer, innerPath: inner, startX, startY, startNormalX, startNormalY, sectorMarkers, toScreen, ptAtP }
}

const RESAMPLE_N = 512

// ─── Widget ───────────────────────────────────────────────────────────────────

export function TrackMap({ snap, playerCarIdx, info }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const animRef   = useRef<number>(0)
  const dataRef   = useRef({ snap, playerCarIdx })
  const cacheRef  = useRef<ShapeCache | null>(null)
  const interpRef = useRef<{
    prevSnap: TrackMapSnapshot | null
    prevTs: number
    currSnap: TrackMapSnapshot | null
    currTs: number
  }>({ prevSnap: null, prevTs: 0, currSnap: null, currTs: 0 })

  const [trackWidth,   setTrackWidth]   = useState(() => loadNum(TRACK_W_KEY, 8, 2, 24))
  const [carRadius,    setCarRadius]    = useState(() => loadNum(CAR_R_KEY, 7, 3, 16))
  const [sfLength,     setSfLength]     = useState(() => loadNum(SF_LEN_KEY, 12, 4, 40))
  const [sectorShow,   setSectorShow]   = useState(() => {
    try { return localStorage.getItem(SECTOR_SHOW_KEY) !== 'false' } catch { return true }
  })
  const [sectorLength, setSectorLength] = useState(() => loadNum(SECTOR_LEN_KEY, 12, 4, 40))
  const [fontSize,     setFontSize]     = useState(() => loadNum(FONT_SIZE_KEY, 12, 8, 20))
  const [showSettings, setShowSettings] = useState(false)

  const settingsRef = useRef<DrawSettings>({ trackWidth, carRadius, sfLength, sectorShow, sectorLength })
  settingsRef.current = { trackWidth, carRadius, sfLength, sectorShow, sectorLength }

  // Keep latest props accessible in the rAF loop without re-scheduling.
  // Detect new snapshots (reference change) and update interpolation state.
  if (snap !== interpRef.current.currSnap) {
    interpRef.current.prevSnap = interpRef.current.currSnap
    interpRef.current.prevTs   = interpRef.current.currTs
    interpRef.current.currSnap = snap
    interpRef.current.currTs   = performance.now()
  }
  dataRef.current = { snap, playerCarIdx }

  useEffect(() => {
    function draw() {
      const canvas = canvasRef.current
      if (!canvas) { animRef.current = requestAnimationFrame(draw); return }
      const ctx = canvas.getContext('2d')
      if (!ctx)   { animRef.current = requestAnimationFrame(draw); return }

      const w = canvas.clientWidth
      const h = canvas.clientHeight
      if (w === 0 || h === 0) { animRef.current = requestAnimationFrame(draw); return }
      if (canvas.width !== w || canvas.height !== h) {
        canvas.width  = w
        canvas.height = h
        cacheRef.current = null  // force path rebuild on resize
      }

      ctx.fillStyle = '#0a0a0a'
      ctx.fillRect(0, 0, w, h)

      const { snap, playerCarIdx } = dataRef.current
      const settings = settingsRef.current

      if (!snap) {
        drawOverlay(ctx, w, h, 'Waiting for first frame…')
        animRef.current = requestAnimationFrame(draw)
        return
      }

      const { shape, cars } = snap

      // Compute interpolation factor: 1.0 = current snapshot, >1.0 = extrapolation.
      const { prevSnap, prevTs, currTs } = interpRef.current
      const now = performance.now()
      const interval = currTs - prevTs
      const t = prevSnap !== null && interval > 16
        ? Math.min(1.8, (now - prevTs) / interval)
        : 1
      const prevCars = prevSnap?.cars ?? cars

      if (!shape) {
        drawTrackPlaceholder(ctx, w, h, cars, prevCars, t, snap.playerCarIdx, settings)
        drawOverlay(ctx, w, h, 'Recording lap…  drive a clean lap to capture the track')
        animRef.current = requestAnimationFrame(draw)
        return
      }

      // Rebuild shape cache on first render or after track/size/sector change.
      const sectors = snap.sectors ?? []
      const cacheKey = `${shape.trackKey}:${w}:${h}:${sectors.join(',')}`
      if (!cacheRef.current || cacheRef.current.key !== cacheKey) {
        cacheRef.current = buildShapeCache(shape, sectors, w, h, cacheKey)
      }
      const cache = cacheRef.current

      drawTrackShape(ctx, cache, settings)
      drawCars(ctx, cache, cars, prevCars, t, playerCarIdx ?? snap.playerCarIdx, settings)

      animRef.current = requestAnimationFrame(draw)
    }

    animRef.current = requestAnimationFrame(draw)
    return () => cancelAnimationFrame(animRef.current)
  }, [])

  function persist(key: string, v: number | boolean) {
    try { localStorage.setItem(key, String(v)) } catch {}
  }

  function handleResetAll() {
    setTrackWidth(8);    persist(TRACK_W_KEY, 8)
    setCarRadius(7);     persist(CAR_R_KEY, 7)
    setSfLength(12);     persist(SF_LEN_KEY, 12)
    setSectorShow(true); persist(SECTOR_SHOW_KEY, true)
    setSectorLength(12); persist(SECTOR_LEN_KEY, 12)
    setFontSize(12);     persist(FONT_SIZE_KEY, 12)
  }

  return (
    <section className="card">
      <h2 style={{ display: 'flex', alignItems: 'center' }}>
        Track Map
        <button
          className={`header-btn${showSettings ? ' header-btn-active' : ''}`}
          onClick={() => setShowSettings(s => !s)}
          title="Settings"
          style={{ marginLeft: 'auto' }}
        >⚙</button>
      </h2>
      <div
        className="card-body"
        style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}
      >
        <SettingsDrawer
          open={showSettings}
          onClose={() => setShowSettings(false)}
          title="Track Map settings"
        >
          <TrackMapSettings
            trackWidth={trackWidth}
            carRadius={carRadius}
            sfLength={sfLength}
            sectorShow={sectorShow}
            sectorLength={sectorLength}
            fontSize={fontSize}
            onTrackWidth={v => { setTrackWidth(v); persist(TRACK_W_KEY, v) }}
            onCarRadius={v => { setCarRadius(v); persist(CAR_R_KEY, v) }}
            onSfLength={v => { setSfLength(v); persist(SF_LEN_KEY, v) }}
            onSectorShow={v => { setSectorShow(v); persist(SECTOR_SHOW_KEY, v) }}
            onSectorLength={v => { setSectorLength(v); persist(SECTOR_LEN_KEY, v) }}
            onFontSize={v => { setFontSize(v); persist(FONT_SIZE_KEY, v) }}
            onResetAll={handleResetAll}
          />
        </SettingsDrawer>
        <div style={{ position: 'relative', flex: '1 1 0', minHeight: 0 }}>
          <canvas
            ref={canvasRef}
            style={{ display: 'block', width: '100%', height: '100%' }}
          />
          <TrackInfoOverlay info={info} fontSize={fontSize} />
        </div>
      </div>
    </section>
  )
}

// ─── Drawing helpers ──────────────────────────────────────────────────────────

function drawOverlay(ctx: CanvasRenderingContext2D, w: number, h: number, text: string) {
  ctx.save()
  ctx.fillStyle = 'rgba(0,0,0,0.55)'
  ctx.fillRect(0, 0, w, h)
  ctx.fillStyle = '#888'
  ctx.font = `${Math.max(10, Math.min(14, w / 24))}px system-ui, sans-serif`
  ctx.textAlign = 'center'
  ctx.textBaseline = 'middle'
  ctx.fillText(text, w / 2, h / 2)
  ctx.restore()
}

function drawTrackShape(ctx: CanvasRenderingContext2D, cache: ShapeCache, s: DrawSettings) {
  // Outer asphalt strip.
  ctx.strokeStyle = '#2a2a2a'
  ctx.lineWidth = s.trackWidth
  ctx.lineJoin = 'round'
  ctx.stroke(cache.outerPath)

  // Centre line — proportional to track width.
  ctx.strokeStyle = '#3a3a3a'
  ctx.lineWidth = Math.max(1, s.trackWidth * 0.25)
  ctx.stroke(cache.innerPath)

  // Start/finish line — perpendicular to the track tangent at p=0.
  const half = s.sfLength / 2
  const nx = cache.startNormalX
  const ny = cache.startNormalY
  ctx.save()
  ctx.lineCap = 'butt'
  ctx.lineWidth = 3
  // White half on the −normal side.
  ctx.strokeStyle = '#ffffff'
  ctx.beginPath()
  ctx.moveTo(cache.startX - nx * half, cache.startY - ny * half)
  ctx.lineTo(cache.startX, cache.startY)
  ctx.stroke()
  // Red half on the +normal side.
  ctx.strokeStyle = '#e22'
  ctx.beginPath()
  ctx.moveTo(cache.startX, cache.startY)
  ctx.lineTo(cache.startX + nx * half, cache.startY + ny * half)
  ctx.stroke()
  ctx.restore()

  // Sector lines (yellow, perpendicular at each SectorStartPct).
  if (s.sectorShow && cache.sectorMarkers.length > 0) {
    const sh = s.sectorLength / 2
    ctx.save()
    ctx.lineCap = 'butt'
    ctx.lineWidth = 2
    ctx.strokeStyle = '#facc15'
    ctx.fillStyle = '#facc15'
    ctx.font = `bold ${Math.max(8, sh * 0.7)}px system-ui, sans-serif`
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    for (const m of cache.sectorMarkers) {
      ctx.beginPath()
      ctx.moveTo(m.x - m.nx * sh, m.y - m.ny * sh)
      ctx.lineTo(m.x + m.nx * sh, m.y + m.ny * sh)
      ctx.stroke()
      // Label at the +normal end.
      const lx = m.x + m.nx * (sh + 6)
      const ly = m.y + m.ny * (sh + 6)
      ctx.fillText(`S${m.num}`, lx, ly)
    }
    ctx.restore()
  }
}

function drawTrackPlaceholder(
  ctx: CanvasRenderingContext2D,
  w: number,
  h: number,
  cars: TrackCar[],
  prevCars: TrackCar[],
  t: number,
  playerIdx: number,
  s: DrawSettings,
) {
  // Fallback ring when shape is not yet recorded.
  const cx = w / 2
  const cy = h / 2
  const r  = Math.min(w, h) * 0.35
  ctx.strokeStyle = '#2a2a2a'
  ctx.lineWidth = s.trackWidth
  ctx.beginPath()
  ctx.arc(cx, cy, r, 0, Math.PI * 2)
  ctx.stroke()

  const ptAtP = (p: number): [number, number] => {
    const angle = p * Math.PI * 2 - Math.PI / 2
    return [cx + Math.cos(angle) * r, cy + Math.sin(angle) * r]
  }

  const prevMap = new Map(prevCars.map((c) => [c.carIdx, c.lapDistPct]))

  // Draw non-player cars first.
  for (const car of cars) {
    if (car.carIdx === playerIdx) continue
    if (car.surface === -1) continue
    const pct = lerpPct(prevMap.get(car.carIdx) ?? car.lapDistPct, car.lapDistPct, t)
    const [sx, sy] = ptAtP(pct)
    drawCarDot(ctx, sx, sy, car, false, s.carRadius)
  }
  // Player on top.
  for (const car of cars) {
    if (car.carIdx !== playerIdx) continue
    const pct = lerpPct(prevMap.get(car.carIdx) ?? car.lapDistPct, car.lapDistPct, t)
    const [sx, sy] = ptAtP(pct)
    drawCarDot(ctx, sx, sy, car, true, s.carRadius)
  }
}

function drawCars(
  ctx: CanvasRenderingContext2D,
  cache: ShapeCache,
  cars: TrackCar[],
  prevCars: TrackCar[],
  t: number,
  playerIdx: number,
  s: DrawSettings,
) {
  const prevMap = new Map(prevCars.map((c) => [c.carIdx, c.lapDistPct]))

  // OnTrack (surface=3) cars rendered last so they appear on top.
  const sorted = [...cars].sort((a, b) => {
    const sa = a.carIdx === playerIdx ? 10 : (a.surface === 3 ? 1 : 0)
    const sb = b.carIdx === playerIdx ? 10 : (b.surface === 3 ? 1 : 0)
    return sa - sb
  })

  for (const car of sorted) {
    if (car.surface === -1) continue
    const pct = lerpPct(prevMap.get(car.carIdx) ?? car.lapDistPct, car.lapDistPct, t)
    const [sx, sy] = cache.ptAtP(pct)
    drawCarDot(ctx, sx, sy, car, car.carIdx === playerIdx, s.carRadius)
  }
}

// ─── Track info overlay ───────────────────────────────────────────────────────

function TrackInfoOverlay({ info, fontSize }: { info: SessionInfoYaml | null; fontSize: number }) {
  const wi = info?.WeekendInfo
  if (!wi) return null

  const name = wi.TrackDisplayName || wi.TrackName
  const config = wi.TrackConfigName?.trim()

  const details: string[] = []
  const location = [wi.TrackCity, wi.TrackCountry].filter(Boolean).join(', ')
  if (location) details.push(location)
  if (wi.TrackAltitude) details.push(wi.TrackAltitude)
  if (wi.TrackLength) details.push(wi.TrackLength)
  if (wi.TrackNumTurns) details.push(`${wi.TrackNumTurns} turns`)
  if (wi.TrackPitSpeedLimit) details.push(`Pit ${wi.TrackPitSpeedLimit}`)

  return (
    <div className="track-info-overlay">
      <div className="track-info-name" style={{ fontSize }}>
        {name}{config ? <span className="track-info-config"> — {config}</span> : null}
      </div>
      {details.length > 0 && (
        <div className="track-info-details" style={{ fontSize: Math.max(8, fontSize - 2) }}>
          {details.join(' · ')}
        </div>
      )}
    </div>
  )
}

function drawCarDot(
  ctx: CanvasRenderingContext2D,
  sx: number,
  sy: number,
  car: TrackCar,
  isPlayer: boolean,
  r: number,
) {
  const alpha = car.onPitRoad ? 0.4 : 1.0
  const color = isPlayer ? '#facc15' : classColor(car.classColor)

  ctx.save()
  ctx.globalAlpha = alpha

  if (isPlayer) {
    // White outline for player car.
    ctx.beginPath()
    ctx.arc(sx, sy, r + 2, 0, Math.PI * 2)
    ctx.fillStyle = '#ffffff'
    ctx.fill()
  }

  ctx.beginPath()
  ctx.arc(sx, sy, r, 0, Math.PI * 2)
  ctx.fillStyle = color
  ctx.fill()

  // Class position label inside the dot (only when big enough).
  if (r >= 7 && car.classPosition > 0) {
    ctx.fillStyle = '#000000'
    ctx.font = `bold ${Math.round(r * 1.1)}px system-ui, sans-serif`
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    ctx.fillText(String(car.classPosition), sx, sy + 0.5)
  }

  ctx.restore()
}
