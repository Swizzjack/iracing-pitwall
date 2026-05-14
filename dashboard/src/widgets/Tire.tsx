import { useState } from 'react'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import { SettingsDrawer } from '../components/SettingsDrawer'
import {
  TireSettings,
  DEFAULT_FIELD_ORDER, DEFAULT_HIDDEN,
  type TireFieldId, type TireStyle, type TempUnit, type PressUnit,
} from './TireSettings'
import { TireBento } from './Tire.bento'
import { TireList } from './Tire.list'
import { TireCluster } from './Tire.cluster'

// ─── Persistence ──────────────────────────────────────────────────────────────

const FIELDS_KEY = 'iracing-tire-fields-v1'
const PREFS_KEY = 'iracing-tire-prefs-v1'

function loadFields(): { order: TireFieldId[]; hidden: Set<TireFieldId> } {
  try {
    const raw = localStorage.getItem(FIELDS_KEY)
    if (raw) {
      const p = JSON.parse(raw)
      const order = Array.isArray(p.order) ? p.order as TireFieldId[] : DEFAULT_FIELD_ORDER
      const hidden = new Set<TireFieldId>(Array.isArray(p.hidden) ? p.hidden : DEFAULT_HIDDEN)
      return { order, hidden }
    }
  } catch { /* ignore */ }
  return { order: DEFAULT_FIELD_ORDER, hidden: new Set(DEFAULT_HIDDEN) }
}

function loadPrefs(): { style: TireStyle; fontScale: number; tempUnit: TempUnit; pressUnit: PressUnit } {
  try {
    const raw = localStorage.getItem(PREFS_KEY)
    if (raw) {
      const p = JSON.parse(raw)
      const style = (['bento', 'list', 'cluster'] as TireStyle[]).includes(p.style) ? p.style as TireStyle : 'bento'
      const tempUnit = (['C', 'F'] as TempUnit[]).includes(p.tempUnit) ? p.tempUnit as TempUnit : 'C'
      const pressUnit = (['psi', 'bar', 'kPa'] as PressUnit[]).includes(p.pressUnit) ? p.pressUnit as PressUnit : 'psi'
      return { style, fontScale: typeof p.fontScale === 'number' ? p.fontScale : 1, tempUnit, pressUnit }
    }
  } catch { /* ignore */ }
  return { style: 'bento', fontScale: 1, tempUnit: 'C', pressUnit: 'psi' }
}

function saveFields(order: TireFieldId[], hidden: Set<TireFieldId>) {
  try { localStorage.setItem(FIELDS_KEY, JSON.stringify({ order, hidden: [...hidden] })) } catch { /* ignore */ }
}

function savePrefs(style: TireStyle, fontScale: number, tempUnit: TempUnit, pressUnit: PressUnit) {
  try { localStorage.setItem(PREFS_KEY, JSON.stringify({ style, fontScale, tempUnit, pressUnit })) } catch { /* ignore */ }
}

// ─── Data Model ───────────────────────────────────────────────────────────────

export interface CornerData {
  label: string
  carcassTemps: [number, number, number]
  surfaceTemps: [number, number, number] | null
  coldPressure: number     // PSI
  hotPressure: number | null  // PSI
  wear: [number, number, number]  // fraction 0–1
  wheelSpeed: number | null  // m/s
  rideHeight: number | null  // meters
}

export function resolveCorners(snap: TelemetrySnapshot): CornerData[] {
  return [
    {
      label: 'LF',
      carcassTemps: snap.tireTempLf,
      surfaceTemps: snap.tireTempSurfaceLf,
      coldPressure: snap.tireColdPressure[0],
      hotPressure: snap.tirePressure ? snap.tirePressure[0] : null,
      wear: snap.tireWearLf,
      wheelSpeed: snap.tireSpeed ? snap.tireSpeed[0] : null,
      rideHeight: snap.tireRideHeight ? snap.tireRideHeight[0] : null,
    },
    {
      label: 'RF',
      carcassTemps: snap.tireTempRf,
      surfaceTemps: snap.tireTempSurfaceRf,
      coldPressure: snap.tireColdPressure[1],
      hotPressure: snap.tirePressure ? snap.tirePressure[1] : null,
      wear: snap.tireWearRf,
      wheelSpeed: snap.tireSpeed ? snap.tireSpeed[1] : null,
      rideHeight: snap.tireRideHeight ? snap.tireRideHeight[1] : null,
    },
    {
      label: 'LR',
      carcassTemps: snap.tireTempLr,
      surfaceTemps: snap.tireTempSurfaceLr,
      coldPressure: snap.tireColdPressure[2],
      hotPressure: snap.tirePressure ? snap.tirePressure[2] : null,
      wear: snap.tireWearLr,
      wheelSpeed: snap.tireSpeed ? snap.tireSpeed[2] : null,
      rideHeight: snap.tireRideHeight ? snap.tireRideHeight[2] : null,
    },
    {
      label: 'RR',
      carcassTemps: snap.tireTempRr,
      surfaceTemps: snap.tireTempSurfaceRr,
      coldPressure: snap.tireColdPressure[3],
      hotPressure: snap.tirePressure ? snap.tirePressure[3] : null,
      wear: snap.tireWearRr,
      wheelSpeed: snap.tireSpeed ? snap.tireSpeed[3] : null,
      rideHeight: snap.tireRideHeight ? snap.tireRideHeight[3] : null,
    },
  ]
}

// ─── Unit helpers (exported for sub-components) ───────────────────────────────

export function fmtTemp(c: number, unit: TempUnit): string {
  if (unit === 'F') return `${(c * 9 / 5 + 32).toFixed(1)}°F`
  return `${c.toFixed(1)}°C`
}

export function fmtTempShort(c: number, unit: TempUnit): string {
  if (unit === 'F') return `${Math.round(c * 9 / 5 + 32)}°`
  return `${c.toFixed(0)}°`
}

export function fmtPress(psi: number, unit: PressUnit): string {
  if (unit === 'bar') return `${(psi * 0.0689476).toFixed(2)} bar`
  if (unit === 'kPa') return `${(psi * 6.89476).toFixed(1)} kPa`
  return `${psi.toFixed(1)} psi`
}

export function tempColor(c: number): string {
  if (c < 60) return '#3b82f6'
  if (c < 80) return '#facc15'
  if (c < 105) return '#22c55e'
  if (c < 115) return '#f97316'
  return '#ef4444'
}

export function wearColor(frac: number): string {
  if (frac > 0.8) return '#22c55e'
  if (frac > 0.5) return '#facc15'
  if (frac > 0.2) return '#f97316'
  return '#ef4444'
}

export function pressDeltaColor(deltaPsi: number): string {
  if (deltaPsi < 0.5) return '#3b82f6'
  if (deltaPsi < 1.5) return '#facc15'
  if (deltaPsi < 3.0) return '#22c55e'
  if (deltaPsi < 4.0) return '#f97316'
  return '#ef4444'
}

// ─── Widget ───────────────────────────────────────────────────────────────────

interface Props {
  snap: TelemetrySnapshot | null
}

export function Tire({ snap }: Props) {
  const [showSettings, setShowSettings] = useState(false)
  const [order, setOrder] = useState<TireFieldId[]>(() => loadFields().order)
  const [hidden, setHidden] = useState<Set<TireFieldId>>(() => loadFields().hidden)
  const [style, setStyle] = useState<TireStyle>(() => loadPrefs().style)
  const [fontScale, setFontScale] = useState(() => loadPrefs().fontScale)
  const [tempUnit, setTempUnit] = useState<TempUnit>(() => loadPrefs().tempUnit)
  const [pressUnit, setPressUnit] = useState<PressUnit>(() => loadPrefs().pressUnit)

  function handleOrderChange(o: TireFieldId[]) {
    setOrder(o)
    saveFields(o, hidden)
  }

  function handleToggle(id: TireFieldId) {
    const next = new Set(hidden)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    setHidden(next)
    saveFields(order, next)
  }

  function handleStyleChange(s: TireStyle) {
    setStyle(s)
    savePrefs(s, fontScale, tempUnit, pressUnit)
  }

  function handleFontScale(s: number) {
    setFontScale(s)
    savePrefs(style, s, tempUnit, pressUnit)
  }

  function handleTempUnit(u: TempUnit) {
    setTempUnit(u)
    savePrefs(style, fontScale, u, pressUnit)
  }

  function handlePressUnit(u: PressUnit) {
    setPressUnit(u)
    savePrefs(style, fontScale, tempUnit, u)
  }

  function handleResetAll() {
    const ord = DEFAULT_FIELD_ORDER
    const hid = new Set(DEFAULT_HIDDEN)
    setOrder(ord)
    setHidden(hid)
    setStyle('bento')
    setFontScale(1)
    setTempUnit('C')
    setPressUnit('psi')
    saveFields(ord, hid)
    savePrefs('bento', 1, 'C', 'psi')
  }

  if (!snap) {
    return (
      <section className="card">
        <h2>Tires</h2>
        <p className="muted">waiting for first frame…</p>
      </section>
    )
  }

  const corners = resolveCorners(snap)
  const visibleOrder = order.filter(id => !hidden.has(id))

  return (
    <section className="card" style={{ '--widget-font-scale-local': fontScale } as React.CSSProperties}>
      <h2 style={{ display: 'flex', alignItems: 'center' }}>
        Tires
        <button
          className={`header-btn${showSettings ? ' header-btn-active' : ''}`}
          onClick={() => setShowSettings(v => !v)}
          title="Settings"
          style={{ marginLeft: 'auto' }}
        >⚙</button>
      </h2>

      <div className="card-body">
        {style === 'bento' && (
          <TireBento corners={corners} fieldOrder={visibleOrder} tempUnit={tempUnit} pressUnit={pressUnit} fontScale={fontScale} />
        )}
        {style === 'list' && (
          <TireList corners={corners} fieldOrder={visibleOrder} tempUnit={tempUnit} pressUnit={pressUnit} fontScale={fontScale} />
        )}
        {style === 'cluster' && (
          <TireCluster corners={corners} fieldOrder={visibleOrder} tempUnit={tempUnit} pressUnit={pressUnit} fontScale={fontScale} />
        )}
      </div>

      <SettingsDrawer open={showSettings} onClose={() => setShowSettings(false)} title="Tires">
        <TireSettings
          order={order}
          hidden={hidden}
          style={style}
          fontScale={fontScale}
          tempUnit={tempUnit}
          pressUnit={pressUnit}
          onOrderChange={handleOrderChange}
          onToggle={handleToggle}
          onStyleChange={handleStyleChange}
          onFontScale={handleFontScale}
          onTempUnit={handleTempUnit}
          onPressUnit={handlePressUnit}
          onResetAll={handleResetAll}
        />
      </SettingsDrawer>
    </section>
  )
}
