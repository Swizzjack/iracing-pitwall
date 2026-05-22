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

const TRACK_W_KEY     = 'iracing-trackmap-track-width'
const CAR_R_KEY       = 'iracing-trackmap-car-size'
const SF_LEN_KEY      = 'iracing-trackmap-sf-length'
const SECTOR_SHOW_KEY = 'iracing-trackmap-sector-show'
const SECTOR_LEN_KEY  = 'iracing-trackmap-sector-length'
const FONT_SIZE_KEY   = 'iracing-trackmap-font-size'
const MODE3D_KEY      = 'iracing-trackmap-mode3d'
const TILT_KEY        = 'iracing-trackmap-tilt'
const ZEXAG_KEY       = 'iracing-trackmap-zexag'

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
  mode3d: boolean
  tiltDeg: number
  zExag: number
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

// ─── Elevation color helper ───────────────────────────────────────────────────

function elevColor(t: number): string {
  const h = Math.round(220 - t * 190)  // 220° blue → 30° orange
  const s = Math.round(28 + t * 10)    // 28 → 38% (vorher 70–90, jetzt dezenter)
  const l = Math.round(52 + t * 6)     // 52 → 58%
  return `hsl(${h},${s}%,${l}%)`
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
  toScreen: (x: number, y: number, z?: number) => [number, number]
  ptAtP: (p: number) => [number, number, number]  // [sx, sy, zWorld]
  // 3D visual cues — empty/zero in 2D mode
  pts3d: [number, number][]  // pre-projected screen coords per track point
  shadowPath: Path2D          // track footprint at z=zMin
  segmentColors: string[]     // per-segment elevation color
  gridSegs: number[]          // flat [x1,y1,x2,y2,alpha, …] for ground grid
  zMin: number
  zExagFactor: number         // zExag * s * sinT — converts Δz to Δpx for poles
}

function buildShapeCache(
  shape: TrackShape,
  sectors: number[],
  w: number,
  h: number,
  key: string,
  mode3d: boolean,
  tiltDeg: number,
  zExag: number,
): ShapeCache {
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
      ptAtP: () => [w / 2, h / 2, 0],
      pts3d: [],
      shadowPath: new Path2D(),
      segmentColors: [],
      gridSegs: [],
      zMin: 0,
      zExagFactor: 0,
    }
    return noop
  }

  // Precompute 3D projection constants.
  const rad  = mode3d ? tiltDeg * Math.PI / 180 : 0
  const cosT = Math.cos(rad)  // y-compression factor; 1 in 2D mode
  const sinT = Math.sin(rad)  // z-lift factor; 0 in 2D mode
  const zBounds = shape.zBounds ?? [0, 0]
  const zmid = (zBounds[0] + zBounds[1]) / 2


  const s = Math.min((w - PAD * 2) / rw, (h - PAD * 2) / rh)
  const cx = w / 2 - ((xmin + xmax) / 2) * s
  // Center the projected track: Y is compressed by cosT so the centroid shifts.
  const cy = h / 2 + ((ymin + ymax) / 2) * s * cosT

  const toScreen = (x: number, y: number, z = 0): [number, number] => [
    cx + x * s,
    cy - y * s * cosT - (z - zmid) * zExag * s * sinT,
  ]

  const pts = shape.points
  const n = pts.length

  // Binary search for interpolated screen position + z at lap dist pct p.
  const ptAtP = (p: number): [number, number, number] => {
    if (n === 0) return [w / 2, h / 2, 0]
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
    const wx = a.x + (b.x - a.x) * t
    const wy = a.y + (b.y - a.y) * t
    const wz = a.z + (b.z - a.z) * t
    const [sx, sy] = toScreen(wx, wy, wz)
    return [sx, sy, wz]
  }

  // Pre-project all track points and build paths in a single pass.
  const pts3d: [number, number][] = new Array(n)
  const outer = new Path2D()
  const inner = new Path2D()
  for (let i = 0; i < n; i++) {
    const sc = toScreen(pts[i].x, pts[i].y, pts[i].z)
    pts3d[i] = sc
    if (i === 0) { outer.moveTo(sc[0], sc[1]); inner.moveTo(sc[0], sc[1]) }
    else          { outer.lineTo(sc[0], sc[1]); inner.lineTo(sc[0], sc[1]) }
  }
  outer.closePath()
  inner.closePath()

  // 3D cues: elevation gradient colors, shadow path at z=zMin.
  const zBoundsArr = shape.zBounds ?? [0, 0]
  const zMin = zBoundsArr[0]
  const zMax = zBoundsArr[1]
  const zRange = zMax - zMin
  const zExagFactor = mode3d ? zExag * s * sinT : 0

  const segmentColors: string[] = mode3d
    ? pts.map(pt => {
        const t = zRange > 0 ? Math.max(0, Math.min(1, (pt.z - zMin) / zRange)) : 0
        return elevColor(t)
      })
    : []

  const shadowPath = new Path2D()
  if (mode3d) {
    for (let i = 0; i < n; i++) {
      const [sx, sy] = toScreen(pts[i].x, pts[i].y, zMin)
      if (i === 0) shadowPath.moveTo(sx, sy)
      else shadowPath.lineTo(sx, sy)
    }
    shadowPath.closePath()
  }

  // Ground grid on the z=zMin plane with radial alpha fade (3D mode only).
  const gridSegs: number[] = []
  if (mode3d) {
    const cxw  = (xmin + xmax) / 2
    const cyw  = (ymin + ymax) / 2
    const extent = Math.max(rw, rh) * 1.5
    const half   = extent / 2
    const target = extent / 10
    const pow    = Math.pow(10, Math.floor(Math.log10(target)))
    const mant   = target / pow
    const spacing = (mant < 2 ? 1 : mant < 5 ? 2 : 5) * pow
    const xStart  = Math.ceil((cxw - half) / spacing) * spacing
    const xEnd    = Math.floor((cxw + half) / spacing) * spacing
    const yStart  = Math.ceil((cyw - half) / spacing) * spacing
    const yEnd    = Math.floor((cyw + half) / spacing) * spacing
    const fadeR   = Math.max(rw, rh) * 0.55
    const SUB     = 24
    const fadeA   = (wx: number, wy: number): number => {
      const d = Math.hypot(wx - cxw, wy - cyw)
      return Math.max(0, 1 - (d / (fadeR * 1.4)) ** 2) * 0.22
    }
    for (let gx = xStart; gx <= xEnd + 1e-6; gx += spacing) {
      for (let i = 0; i < SUB; i++) {
        const wy0 = cyw - half + (i / SUB) * extent
        const wy1 = cyw - half + ((i + 1) / SUB) * extent
        const a   = (fadeA(gx, wy0) + fadeA(gx, wy1)) / 2
        if (a < 0.01) continue
        const [sx0, sy0] = toScreen(gx, wy0, zMin)
        const [sx1, sy1] = toScreen(gx, wy1, zMin)
        gridSegs.push(sx0, sy0, sx1, sy1, a)
      }
    }
    for (let gy = yStart; gy <= yEnd + 1e-6; gy += spacing) {
      for (let i = 0; i < SUB; i++) {
        const wx0 = cxw - half + (i / SUB) * extent
        const wx1 = cxw - half + ((i + 1) / SUB) * extent
        const a   = (fadeA(wx0, gy) + fadeA(wx1, gy)) / 2
        if (a < 0.01) continue
        const [sx0, sy0] = toScreen(wx0, gy, zMin)
        const [sx1, sy1] = toScreen(wx1, gy, zMin)
        gridSegs.push(sx0, sy0, sx1, sy1, a)
      }
    }
  }

  const [startX, startY] = toScreen(pts[0].x, pts[0].y, pts[0].z)

  // Compute the unit normal at p=0 in screen space to draw the S/F line perpendicular
  // to the track tangent. We use the two neighbours of the first point.
  const [ax, ay] = toScreen(pts[(n - 1) % n].x, pts[(n - 1) % n].y, pts[(n - 1) % n].z)
  const [bx, by] = toScreen(pts[1 % n].x, pts[1 % n].y, pts[1 % n].z)
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

  return {
    key, w, h,
    outerPath: outer, innerPath: inner,
    startX, startY, startNormalX, startNormalY,
    sectorMarkers, toScreen, ptAtP,
    pts3d, shadowPath, segmentColors, gridSegs, zMin, zExagFactor,
  }
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
  const [mode3d,       setMode3d]       = useState(() => {
    try { return localStorage.getItem(MODE3D_KEY) === 'true' } catch { return false }
  })
  const [tiltDeg,      setTiltDeg]      = useState(() => loadNum(TILT_KEY, 45, 0, 75))
  const [zExag,        setZExag]        = useState(() => loadNum(ZEXAG_KEY, 10, 1, 50))
  const [showSettings, setShowSettings] = useState(false)

  const settingsRef = useRef<DrawSettings>({ trackWidth, carRadius, sfLength, sectorShow, sectorLength, mode3d, tiltDeg, zExag })
  settingsRef.current = { trackWidth, carRadius, sfLength, sectorShow, sectorLength, mode3d, tiltDeg, zExag }

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

      // Rebuild shape cache on track/size/sector/3D-setting change.
      const sectors = snap.sectors ?? []
      const { mode3d, tiltDeg, zExag } = settings
      const cacheKey = `${shape.trackKey}:${w}:${h}:${sectors.join(',')}:${mode3d?'3':'2'}:${tiltDeg}:${zExag}`
      if (!cacheRef.current || cacheRef.current.key !== cacheKey) {
        cacheRef.current = buildShapeCache(shape, sectors, w, h, cacheKey, mode3d, tiltDeg, zExag)
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
    setTiltDeg(45);      persist(TILT_KEY, 45)
    setZExag(10);        persist(ZEXAG_KEY, 10)
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
            tiltDeg={tiltDeg}
            zExag={zExag}
            onTrackWidth={v => { setTrackWidth(v); persist(TRACK_W_KEY, v) }}
            onCarRadius={v => { setCarRadius(v); persist(CAR_R_KEY, v) }}
            onSfLength={v => { setSfLength(v); persist(SF_LEN_KEY, v) }}
            onSectorShow={v => { setSectorShow(v); persist(SECTOR_SHOW_KEY, v) }}
            onSectorLength={v => { setSectorLength(v); persist(SECTOR_LEN_KEY, v) }}
            onFontSize={v => { setFontSize(v); persist(FONT_SIZE_KEY, v) }}
            onTiltDeg={v => { setTiltDeg(v); persist(TILT_KEY, v) }}
            onZExag={v => { setZExag(v); persist(ZEXAG_KEY, v) }}
            onResetAll={handleResetAll}
          />
        </SettingsDrawer>
        <div style={{ position: 'relative', flex: '1 1 0', minHeight: 0 }}>
          <canvas
            ref={canvasRef}
            style={{ display: 'block', width: '100%', height: '100%' }}
          />
          <button
            className={`header-btn${mode3d ? ' header-btn-active' : ''}`}
            onClick={() => { const next = !mode3d; setMode3d(next); persist(MODE3D_KEY, next) }}
            title={mode3d ? 'Switch to 2D view' : 'Switch to 3D view'}
            style={{ position: 'absolute', top: 8, right: 8, zIndex: 2 }}
          >{mode3d ? '3D' : '2D'}</button>
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
  if (s.mode3d && cache.segmentColors.length > 0) {
    // ── 3D mode ──────────────────────────────────────────────────────────────
    // 0. Ground grid on z=zMin with radial fade.
    const g = cache.gridSegs
    if (g.length > 0) {
      ctx.save()
      ctx.lineWidth = 1
      for (let i = 0; i < g.length; i += 5) {
        ctx.strokeStyle = `rgba(150,160,190,${g[i + 4].toFixed(3)})`
        ctx.beginPath()
        ctx.moveTo(g[i], g[i + 1])
        ctx.lineTo(g[i + 2], g[i + 3])
        ctx.stroke()
      }
      ctx.restore()
    }

    // 1. Ground shadow at z=zMin: thin, muted stroke shows track footprint.
    ctx.save()
    ctx.strokeStyle = 'rgba(170, 180, 210, 0.38)'
    ctx.lineWidth = s.trackWidth * 0.7
    ctx.lineJoin = 'round'
    ctx.stroke(cache.shadowPath)
    ctx.restore()

    // 2. Elevation-colored track — segment by segment (Painter-friendly).
    ctx.save()
    ctx.lineJoin = 'round'
    ctx.lineCap = 'round'
    ctx.lineWidth = s.trackWidth
    const pd = cache.pts3d
    const colors = cache.segmentColors
    const np = pd.length
    for (let i = 0; i < np; i++) {
      const j = (i + 1) % np
      ctx.strokeStyle = colors[i]
      ctx.beginPath()
      ctx.moveTo(pd[i][0], pd[i][1])
      ctx.lineTo(pd[j][0], pd[j][1])
      ctx.stroke()
    }
    ctx.restore()
  } else {
    // ── 2D mode: fast Path2D ─────────────────────────────────────────────────
    // Outer asphalt strip.
    ctx.strokeStyle = '#2a2a2a'
    ctx.lineWidth = s.trackWidth
    ctx.lineJoin = 'round'
    ctx.stroke(cache.outerPath)

    // Centre line — proportional to track width.
    ctx.strokeStyle = '#3a3a3a'
    ctx.lineWidth = Math.max(1, s.trackWidth * 0.25)
    ctx.stroke(cache.innerPath)
  }

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

  // Pre-compute screen positions so we can z-sort before drawing (Painter's algorithm).
  const visible = cars
    .filter(car => car.surface !== -1)
    .map(car => {
      const pct = lerpPct(prevMap.get(car.carIdx) ?? car.lapDistPct, car.lapDistPct, t)
      const [sx, sy, zWorld] = cache.ptAtP(pct)
      const isPlayer = car.carIdx === playerIdx
      return { car, sx, sy, zWorld, isPlayer }
    })

  // OnTrack (surface=3) above off-track; player always on top.
  // In 3D mode also sort by elevation ascending so higher cars render over lower ones.
  visible.sort((a, b) => {
    const pa = a.isPlayer ? 2 : (a.car.surface === 3 ? 1 : 0)
    const pb = b.isPlayer ? 2 : (b.car.surface === 3 ? 1 : 0)
    if (pa !== pb) return pa - pb
    return s.mode3d ? a.zWorld - b.zWorld : 0
  })

  // In 3D mode: draw shadow blobs + depth poles before all car dots
  // so car bodies always render on top regardless of elevation order.
  if (s.mode3d && cache.zExagFactor > 0) {
    for (const { sx, sy, zWorld, isPlayer } of visible) {
      const poleH = (zWorld - cache.zMin) * cache.zExagFactor
      if (poleH < 1) continue
      const shadowY = sy + poleH
      // Shadow blob at ground level.
      ctx.save()
      ctx.globalAlpha = 0.35
      ctx.beginPath()
      ctx.arc(sx, shadowY, s.carRadius * 0.9, 0, Math.PI * 2)
      ctx.fillStyle = isPlayer ? 'rgba(250,204,21,0.6)' : 'rgba(140,160,255,0.5)'
      ctx.fill()
      ctx.restore()
      // Vertical pole connecting car to its shadow.
      ctx.save()
      ctx.globalAlpha = 0.3
      ctx.beginPath()
      ctx.moveTo(sx, sy + s.carRadius)
      ctx.lineTo(sx, shadowY - s.carRadius * 0.7)
      ctx.strokeStyle = isPlayer ? '#facc15' : '#99aadd'
      ctx.lineWidth = 1.5
      ctx.stroke()
      ctx.restore()
    }
  }

  for (const { car, sx, sy, isPlayer } of visible) {
    drawCarDot(ctx, sx, sy, car, isPlayer, s.carRadius)
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
