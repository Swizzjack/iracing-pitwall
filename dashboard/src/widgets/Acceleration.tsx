import { useState } from 'react'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import { SettingsDrawer } from '../components/SettingsDrawer'
import { AccelerationSettings, type SpeedUnit } from './AccelerationSettings'
import { useLaunchTracker, type LaunchResult, type LaunchPhase } from './useLaunchTracker'

// ─── localStorage ─────────────────────────────────────────────────────────────

const PREFS_KEY = 'iracing-acceleration-prefs-v1'

function loadPrefs(): { unit: SpeedUnit; fontScale: number } {
  try {
    const raw = localStorage.getItem(PREFS_KEY)
    if (raw) {
      const p = JSON.parse(raw)
      return {
        unit: p.unit === 'mph' ? 'mph' : 'kmh',
        fontScale: typeof p.fontScale === 'number' ? p.fontScale : 1,
      }
    }
  } catch { /* ignore */ }
  return { unit: 'kmh', fontScale: 1 }
}

function savePrefs(unit: SpeedUnit, fontScale: number) {
  try { localStorage.setItem(PREFS_KEY, JSON.stringify({ unit, fontScale })) } catch { /* ignore */ }
}

// ─── Unit helpers ─────────────────────────────────────────────────────────────

const MS_TO_KMH = 3.6
const MS_TO_MPH = 2.23694

function speedIn(ms: number, unit: SpeedUnit): number {
  return unit === 'mph' ? ms * MS_TO_MPH : ms * MS_TO_KMH
}

function unitLabel(unit: SpeedUnit): string {
  return unit === 'mph' ? 'mph' : 'km/h'
}

// ─── Status badge ─────────────────────────────────────────────────────────────

function StatusBadge({ phase }: { phase: LaunchPhase }) {
  if (phase === 'armed') return <span className="accel-status accel-status-armed">Ready</span>
  if (phase === 'recording') return <span className="accel-status accel-status-recording">Recording</span>
  return null
}

// ─── Live view (while recording) ───────────────────────────────────────────────

function LiveView({ speedMs, elapsed, unit }: { speedMs: number; elapsed: number; unit: SpeedUnit }) {
  return (
    <div className="accel-live">
      <div className="accel-live-speed">{speedIn(speedMs, unit).toFixed(0)}</div>
      <div className="accel-live-unit">{unitLabel(unit)}</div>
      <div className="accel-live-time">{elapsed.toFixed(2)} s</div>
    </div>
  )
}

// ─── Waiting view (no result yet) ──────────────────────────────────────────────

function WaitingView({ phase }: { phase: LaunchPhase }) {
  const text = phase === 'armed'
    ? 'Ready — floor it to start the timer.'
    : 'Come to a complete stop to arm the standing-start timer.'
  return (
    <div className="accel-waiting">
      <p className="muted" style={{ textAlign: 'center' }}>{text}</p>
    </div>
  )
}

// ─── Result graph ─────────────────────────────────────────────────────────────

const GRAPH_W = 400
const GRAPH_H = 100

function graphTimeRange(result: LaunchResult): [number, number] {
  const tMin = result.samples[0]?.t ?? 0
  const tMax = Math.max(result.time100kmh + 0.3, result.samples[result.samples.length - 1]?.t ?? 1)
  return [tMin, tMax]
}

function LaunchGraph({ result, unit }: { result: LaunchResult; unit: SpeedUnit }) {
  const { samples, time100kmh, peakSpeedMs, peakAccelG } = result
  const [tMin, tMax] = graphTimeRange(result)
  const speedMax = Math.max(speedIn(peakSpeedMs, unit), 1) * 1.08
  const accelMin = Math.min(0, ...samples.map(s => s.longAccelG)) * 1.1
  const accelMax = Math.max(peakAccelG, 0.1) * 1.1

  const x = (t: number) => ((t - tMin) / (tMax - tMin)) * GRAPH_W
  const ySpeed = (v: number) => GRAPH_H - (v / speedMax) * GRAPH_H
  const yAccel = (v: number) => GRAPH_H - ((v - accelMin) / (accelMax - accelMin)) * GRAPH_H

  const speedPts = samples.map(s => `${x(s.t).toFixed(1)},${ySpeed(speedIn(s.speedMs, unit)).toFixed(1)}`).join(' ')
  const accelPts = samples.map(s => `${x(s.t).toFixed(1)},${yAccel(s.longAccelG).toFixed(1)}`).join(' ')
  const markerX = x(time100kmh)
  const zeroY = yAccel(0)

  return (
    <svg className="accel-graph" viewBox={`0 0 ${GRAPH_W} ${GRAPH_H}`} preserveAspectRatio="none" aria-hidden="true">
      <line className="accel-zero-line" x1={0} x2={GRAPH_W} y1={zeroY} y2={zeroY} />
      <line className="accel-target-marker" x1={markerX} x2={markerX} y1={0} y2={GRAPH_H} />
      <polyline className="accel-g-line" points={accelPts} />
      <polyline className="accel-speed-line" points={speedPts} />
    </svg>
  )
}

// ─── Result view (last measurement) ────────────────────────────────────────────

function ResultView({ result, unit, onClear }: { result: LaunchResult; unit: SpeedUnit; onClear: () => void }) {
  const primaryTime = unit === 'mph' ? result.time60mph : result.time100kmh
  const primaryLabel = unit === 'mph' ? '0–60 mph' : '0–100 km/h'
  const secondaryTime = unit === 'mph' ? result.time100kmh : result.time60mph
  const secondaryLabel = unit === 'mph' ? '0–100 km/h' : '0–60 mph'
  const [tMin, tMax] = graphTimeRange(result)

  return (
    <div className="accel-result">
      <div className="accel-hero">
        <div className="accel-hero-label">{primaryLabel}</div>
        <div className="accel-hero-value">{primaryTime.toFixed(2)}<span className="accel-hero-unit">s</span></div>
        <div className="accel-secondary">{secondaryLabel}: {secondaryTime.toFixed(2)} s · Peak {result.peakAccelG.toFixed(2)} G</div>
      </div>
      <div className="accel-legend">
        <span><span className="accel-legend-dot" style={{ background: '#f5a623' }} />Speed ({unitLabel(unit)})</span>
        <span><span className="accel-legend-dot" style={{ background: '#3b82f6' }} />Long. G</span>
      </div>
      <LaunchGraph result={result} unit={unit} />
      <div className="accel-axis-row">
        <span>{tMin.toFixed(2)}s</span>
        <span>{tMax.toFixed(2)}s</span>
      </div>
      <button className="accel-clear-btn" onClick={onClear}>Clear result</button>
    </div>
  )
}

// ─── Main widget ──────────────────────────────────────────────────────────────

interface Props {
  snap: TelemetrySnapshot | null
}

export function Acceleration({ snap }: Props) {
  const [showSettings, setShowSettings] = useState(false)
  const [unit, setUnit] = useState<SpeedUnit>(() => loadPrefs().unit)
  const [fontScale, setFontScale] = useState<number>(() => loadPrefs().fontScale)
  const { phase, liveSpeedMs, liveElapsed, lastResult, clearResult } = useLaunchTracker(snap)

  function handleUnit(u: SpeedUnit) { setUnit(u); savePrefs(u, fontScale) }
  function handleFontScale(s: number) { setFontScale(s); savePrefs(unit, s) }
  function handleResetAll() { setUnit('kmh'); setFontScale(1); savePrefs('kmh', 1) }

  if (!snap) {
    return (
      <section className="card">
        <h2>Acceleration</h2>
        <div className="card-body">
          <p className="muted">waiting for first frame…</p>
        </div>
      </section>
    )
  }

  return (
    <section className="card" style={{ '--widget-font-scale-local': fontScale } as React.CSSProperties}>
      <h2 style={{ display: 'flex', alignItems: 'center' }}>
        Acceleration
        <StatusBadge phase={phase} />
        <button
          className={`header-btn${showSettings ? ' header-btn-active' : ''}`}
          style={{ marginLeft: 'auto' }}
          onClick={() => setShowSettings(s => !s)}
        >⚙</button>
      </h2>
      <SettingsDrawer open={showSettings} onClose={() => setShowSettings(false)} title="Acceleration Settings">
        <AccelerationSettings
          unit={unit}
          fontScale={fontScale}
          onUnit={handleUnit}
          onFontScale={handleFontScale}
          onResetAll={handleResetAll}
        />
      </SettingsDrawer>
      <div className="card-body accel-body">
        {phase === 'recording'
          ? <LiveView speedMs={liveSpeedMs} elapsed={liveElapsed} unit={unit} />
          : lastResult
            ? <ResultView result={lastResult} unit={unit} onClear={clearResult} />
            : <WaitingView phase={phase} />}
      </div>
    </section>
  )
}
