import { useRef, useEffect, useState } from 'react'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'

type Channel = 'THR' | 'BRK' | 'CLT' | 'STEER' | 'GX' | 'GZ'

const CHANNEL_COLORS: Record<Channel, string> = {
  THR:   '#22c55e',
  BRK:   '#ef4444',
  CLT:   '#3b82f6',
  STEER: '#eab308',
  GX:    '#f97316',
  GZ:    '#06b6d4',
}

const SPEEDS = [
  { label: '5s',  ms:  5_000 },
  { label: '15s', ms: 15_000 },
  { label: '30s', ms: 30_000 },
]

const ALL_CHANNELS: Channel[] = ['THR', 'BRK', 'CLT', 'STEER', 'GX', 'GZ']
const MAX_SAMPLES   = 1800                       // 30 Hz × 60 s headroom
const STEER_MAX_RAD = (540 * Math.PI) / 180      // ≈ 9.42 rad — Sim-Racing-Default
const MAX_G         = 30                         // m/s² ≈ ±3 g

interface Sample {
  t:     number  // performance.now() ms
  thr:   number  // 0–1
  brk:   number  // 0–1
  clt:   number  // 0–1 (inverted: iRacing 1=released → 0=pressed)
  steer: number  // 0–1 (centred at 0.5 = straight)
  gx:    number  // 0–1 (centred at 0.5 = 0 g lateral)
  gz:    number  // 0–1 (centred at 0.5 = 0 g longitudinal)
}

const GETTERS: Record<Channel, (s: Sample) => number> = {
  THR:   (s) => s.thr,
  BRK:   (s) => s.brk,
  CLT:   (s) => s.clt,
  STEER: (s) => s.steer,
  GX:    (s) => s.gx,
  GZ:    (s) => s.gz,
}

function clamp01(v: number) { return Math.max(0, Math.min(1, v)) }

function loadChannels(): Set<Channel> {
  try {
    const raw = localStorage.getItem('iracing-inputchart-channels')
    if (raw) {
      const parsed = JSON.parse(raw) as string[]
      const valid = parsed.filter((c): c is Channel => ALL_CHANNELS.includes(c as Channel))
      if (valid.length > 0) return new Set(valid)
    }
  } catch { /* ignore */ }
  return new Set<Channel>(['THR', 'BRK', 'CLT', 'STEER'])
}

function loadSpeedIdx(): number {
  try {
    const raw = localStorage.getItem('iracing-inputchart-speed')
    if (raw !== null) {
      const idx = parseInt(raw, 10)
      if (idx >= 0 && idx < SPEEDS.length) return idx
    }
  } catch { /* ignore */ }
  return 1
}

export function TelemetryInputs({ snap }: { snap: TelemetrySnapshot | null }) {
  const canvasRef  = useRef<HTMLCanvasElement>(null)
  const samplesRef = useRef<Sample[]>([])
  const animRef    = useRef<number>(0)

  const [active,   setActive]   = useState<Set<Channel>>(loadChannels)
  const [speedIdx, setSpeedIdx] = useState<number>(loadSpeedIdx)

  // Push one sample per incoming snapshot (≤60 Hz, rAF-coalesced by App.tsx)
  useEffect(() => {
    if (!snap) return
    // Steering: radians → 0..1 centred at 0.5
    const steer = clamp01((snap.steeringWheelAngle / STEER_MAX_RAD + 1) / 2)
    // Clutch: iRacing 1=released, invert so 1=pressed (matches THR/BRK convention)
    const clt = clamp01(1 - snap.clutch)
    // G-forces: ±MAX_G m/s² → 0..1, 0.5 = 0 g
    const gx = clamp01((snap.latAccel  / MAX_G + 1) / 2)
    const gz = clamp01((snap.longAccel / MAX_G + 1) / 2)
    const buf = samplesRef.current
    buf.push({ t: performance.now(), thr: snap.throttle, brk: snap.brake, clt, steer, gx, gz })
    if (buf.length > MAX_SAMPLES) buf.splice(0, buf.length - MAX_SAMPLES)
  }, [snap])

  // Canvas render loop
  useEffect(() => {
    const windowMs = SPEEDS[speedIdx].ms

    function draw() {
      const canvas = canvasRef.current
      if (!canvas) { animRef.current = requestAnimationFrame(draw); return }
      const ctx = canvas.getContext('2d')
      if (!ctx)   { animRef.current = requestAnimationFrame(draw); return }
      const w = canvas.clientWidth
      const h = canvas.clientHeight
      if (w === 0 || h === 0) { animRef.current = requestAnimationFrame(draw); return }
      if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h }

      ctx.fillStyle = '#0a0a0a'
      ctx.fillRect(0, 0, w, h)

      // Horizontal grid lines at 25 / 50 / 75 %
      ctx.strokeStyle = '#1e1e1e'
      ctx.lineWidth   = 1
      for (const pct of [0.25, 0.5, 0.75]) {
        const y = Math.round(pct * h) + 0.5
        ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(w, y); ctx.stroke()
      }

      const now   = performance.now()
      const start = now - windowMs
      const vis   = samplesRef.current.filter((s) => s.t >= start)

      if (vis.length >= 2) {
        const xFor = (t: number) => ((t - start) / windowMs) * w
        for (const ch of ALL_CHANNELS) {
          if (!active.has(ch)) continue
          const get = GETTERS[ch]
          ctx.beginPath()
          ctx.strokeStyle = CHANNEL_COLORS[ch]
          ctx.lineWidth   = 1.5
          ctx.lineJoin    = 'round'
          let first = true
          for (const s of vis) {
            const x = xFor(s.t)
            const y = (1 - get(s)) * h
            if (first) { ctx.moveTo(x, y); first = false } else ctx.lineTo(x, y)
          }
          ctx.stroke()
        }
      }

      animRef.current = requestAnimationFrame(draw)
    }

    animRef.current = requestAnimationFrame(draw)
    return () => cancelAnimationFrame(animRef.current)
  }, [active, speedIdx])

  function toggleChannel(ch: Channel) {
    setActive((prev) => {
      const next = new Set(prev)
      if (next.has(ch)) { next.delete(ch) } else { next.add(ch) }
      try { localStorage.setItem('iracing-inputchart-channels', JSON.stringify([...next])) } catch { /* ignore */ }
      return next
    })
  }

  function selectSpeed(idx: number) {
    setSpeedIdx(idx)
    try { localStorage.setItem('iracing-inputchart-speed', String(idx)) } catch { /* ignore */ }
  }

  const mono = 'ui-monospace, SFMono-Regular, Menlo, monospace'

  return (
    <section className="card">
      <h2>Telemetry Inputs</h2>
      <div className="card-body" style={{ display: 'flex', flexDirection: 'column', gap: 8, overflow: 'hidden' }}>
        <div style={{ position: 'relative', flex: '1 1 0', minHeight: 0 }}>
          <canvas
            ref={canvasRef}
            style={{ display: 'block', width: '100%', height: '100%', borderRadius: 3 }}
          />
          {snap === null && (
            <p
              className="muted"
              style={{
                position: 'absolute',
                inset: 0,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                margin: 0,
              }}
            >
              waiting for first frame…
            </p>
          )}
        </div>

        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: 8 }}>
          <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
            {ALL_CHANNELS.map((ch) => {
              const on = active.has(ch)
              return (
                <button
                  key={ch}
                  onClick={() => toggleChannel(ch)}
                  style={{
                    background:    on ? CHANNEL_COLORS[ch] + '22' : 'transparent',
                    border:        `1px solid ${on ? CHANNEL_COLORS[ch] : '#333'}`,
                    color:         on ? CHANNEL_COLORS[ch] : '#555',
                    borderRadius:  3,
                    padding:       '2px 8px',
                    fontSize:      12,
                    fontFamily:    mono,
                    cursor:        'pointer',
                    letterSpacing: '0.05em',
                  }}
                >
                  {ch}
                </button>
              )
            })}
          </div>

          <div style={{ display: 'flex', gap: 3, flexShrink: 0 }}>
            {SPEEDS.map((opt, i) => {
              const on = speedIdx === i
              return (
                <button
                  key={opt.label}
                  onClick={() => selectSpeed(i)}
                  style={{
                    background:   on ? '#ffffff12' : 'transparent',
                    border:       `1px solid ${on ? '#555' : '#2a2a2a'}`,
                    color:        on ? '#ccc' : '#555',
                    borderRadius: 3,
                    padding:      '2px 8px',
                    fontSize:     12,
                    fontFamily:   mono,
                    cursor:       'pointer',
                  }}
                >
                  {opt.label}
                </button>
              )
            })}
          </div>
        </div>
      </div>
    </section>
  )
}
