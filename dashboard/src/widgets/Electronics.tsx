import { useState } from 'react'
import { Cpu } from 'lucide-react'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import { SettingsDrawer } from '../components/SettingsDrawer'
import {
  ElectronicsSettings,
  DEFAULT_FIELD_ORDER, DEFAULT_HIDDEN, FIELD_GROUP,
  type ElectronicsFieldId, type ElectronicsStyle,
} from './ElectronicsSettings'
import { ElectronicsBento } from './Electronics.bento'
import { ElectronicsList } from './Electronics.list'
import { ElectronicsCluster } from './Electronics.cluster'

// ─── localStorage ─────────────────────────────────────────────────────────────

const FIELDS_KEY = 'iracing-electronics-fields-v1'
const PREFS_KEY = 'iracing-electronics-prefs-v1'

function loadFields(): { order: ElectronicsFieldId[]; hidden: Set<ElectronicsFieldId> } {
  try {
    const raw = localStorage.getItem(FIELDS_KEY)
    if (raw) {
      const p = JSON.parse(raw)
      const order = Array.isArray(p.order) ? p.order as ElectronicsFieldId[] : DEFAULT_FIELD_ORDER
      const hidden = new Set<ElectronicsFieldId>(Array.isArray(p.hidden) ? p.hidden : DEFAULT_HIDDEN)
      return { order, hidden }
    }
  } catch { /* ignore */ }
  return { order: DEFAULT_FIELD_ORDER, hidden: new Set(DEFAULT_HIDDEN) }
}

function loadPrefs(): { style: ElectronicsStyle; fontScale: number } {
  try {
    const raw = localStorage.getItem(PREFS_KEY)
    if (raw) {
      const p = JSON.parse(raw)
      const style = (['bento', 'list', 'cluster'] as ElectronicsStyle[]).includes(p.style)
        ? p.style as ElectronicsStyle : 'bento'
      return {
        style,
        fontScale: typeof p.fontScale === 'number' ? p.fontScale : 1,
      }
    }
  } catch { /* ignore */ }
  return { style: 'bento', fontScale: 1 }
}

function saveFields(order: ElectronicsFieldId[], hidden: Set<ElectronicsFieldId>) {
  try { localStorage.setItem(FIELDS_KEY, JSON.stringify({ order, hidden: [...hidden] })) } catch { /* ignore */ }
}

function savePrefs(style: ElectronicsStyle, fontScale: number) {
  try { localStorage.setItem(PREFS_KEY, JSON.stringify({ style, fontScale })) } catch { /* ignore */ }
}

// ─── Resolved field (shared type used by all 3 style components) ──────────────

export interface ResolvedField {
  id: ElectronicsFieldId
  group: 'adjustments' | 'hybrid' | 'engine' | 'status'
  label: string
  value: string
  color?: string
  barPct?: number
}

const FIELD_LABELS: Record<ElectronicsFieldId, string> = {
  tc1: 'TC 1', tc2: 'TC 2', abs: 'ABS', bb: 'Brake Bias',
  throttleShape: 'Throttle Shape', diffEntry: 'Diff Entry', diffMid: 'Diff Mid',
  diffExit: 'Diff Exit', arbFront: 'ARB Front', arbRear: 'ARB Rear',
  engineBraking: 'Engine Braking',
  mgukMode: 'MGU-K Mode', mgukPower: 'MGU-K Power', battery: 'Battery (ERS)',
  drs: 'DRS', p2p: 'Push-to-Pass',
  gear: 'Gear', rpm: 'RPM', speed: 'Speed', waterTemp: 'Water Temp',
  oilTemp: 'Oil Temp', oilPress: 'Oil Press', fuelPress: 'Fuel Press',
  manifoldPress: 'Manifold Press', voltage: 'Voltage',
  engineWarnings: 'Engine Warnings', shiftLights: 'Shift Lights',
}

const MGUK_MODES: Record<number, string> = {
  0: 'Off', 1: 'Low', 2: 'Medium', 3: 'High', 4: 'Hot Lap', 5: 'Time Attack',
}

function resolveField(id: ElectronicsFieldId, snap: TelemetrySnapshot): ResolvedField | null {
  const label = FIELD_LABELS[id]
  const group = FIELD_GROUP[id]
  const r = (value: string, color?: string, barPct?: number): ResolvedField =>
    ({ id, group, label, value, color, barPct })

  switch (id) {
    case 'tc1':
      if (snap.dcTractionControl == null) return null
      return r(String(snap.dcTractionControl > 1.5 ? Math.round(snap.dcTractionControl) : Math.round(snap.dcTractionControl * 12)), '#22c55e')
    case 'tc2':
      if (snap.dcTractionControl2 == null) return null
      return r(String(snap.dcTractionControl2 > 1.5 ? Math.round(snap.dcTractionControl2) : Math.round(snap.dcTractionControl2 * 12)), '#22c55e')
    case 'abs':
      if (snap.dcAbs == null) return null
      return r(String(snap.dcAbs > 1.5 ? Math.round(snap.dcAbs) : Math.round(snap.dcAbs * 12)), '#3b82f6')
    case 'bb': {
      const bbPct = snap.brakeBias > 1 ? snap.brakeBias : snap.brakeBias * 100
      return r(`${bbPct.toFixed(1)}%`)
    }
    case 'throttleShape':
      if (snap.dcThrottleShape == null) return null
      return r(snap.dcThrottleShape.toFixed(1))
    case 'diffEntry':
      if (snap.dcDiffEntry == null) return null
      return r(snap.dcDiffEntry.toFixed(1))
    case 'diffMid':
      if (snap.dcDiffMiddle == null) return null
      return r(snap.dcDiffMiddle.toFixed(1))
    case 'diffExit':
      if (snap.dcDiffExit == null) return null
      return r(snap.dcDiffExit.toFixed(1))
    case 'arbFront':
      if (snap.dcAntiRollFront == null) return null
      return r(snap.dcAntiRollFront.toFixed(1))
    case 'arbRear':
      if (snap.dcAntiRollRear == null) return null
      return r(snap.dcAntiRollRear.toFixed(1))
    case 'engineBraking':
      if (snap.dcEngineBraking == null) return null
      return r(snap.dcEngineBraking.toFixed(1))

    case 'mgukMode':
      if (snap.mgukDeployMode == null) return null
      return r(MGUK_MODES[snap.mgukDeployMode] ?? `Mode ${snap.mgukDeployMode}`, '#3b82f6')
    case 'mgukPower':
      if (snap.powerMguk == null || snap.powerMguk <= 0) return null
      return r(`${(snap.powerMguk / 1000).toFixed(1)} kW`, '#3b82f6')
    case 'battery': {
      if (snap.energyBatteryPct == null) return null
      const pct = snap.energyBatteryPct
      const color = pct > 0.3 ? '#3b82f6' : pct > 0.15 ? '#facc15' : '#ef4444'
      return r(`${(pct * 100).toFixed(0)}%`, color, pct)
    }
    case 'drs':
      if (snap.drsStatus == null) return null
      return r(snap.drsStatus >= 2 ? 'ACTIVE' : snap.drsStatus >= 1 ? 'READY' : 'OFF',
        snap.drsStatus >= 2 ? '#22c55e' : snap.drsStatus >= 1 ? '#facc15' : '#555')
    case 'p2p':
      if (snap.p2pCount == null && snap.p2pStatus == null) return null
      return r(
        `${snap.p2pStatus ? 'ACTIVE' : 'OFF'}${snap.p2pCount != null ? ` ×${snap.p2pCount}` : ''}`,
        snap.p2pStatus ? '#3b82f6' : '#555',
      )

    case 'gear': {
      const g = snap.gear
      const gStr = g === -1 ? 'R' : g === 0 ? 'N' : String(g)
      return r(gStr, g === -1 ? '#ef4444' : g === 0 ? '#facc15' : '#f0f0f0')
    }
    case 'rpm':
      return r(snap.rpm.toFixed(0))
    case 'speed':
      return r(`${(snap.speedMs * 3.6).toFixed(0)} km/h`)
    case 'waterTemp': {
      const c = snap.waterTemp > 105 ? '#ef4444' : snap.waterTemp > 95 ? '#facc15' : '#22c55e'
      return r(`${snap.waterTemp.toFixed(1)} °C`, c, Math.min(snap.waterTemp / 120, 1))
    }
    case 'oilTemp': {
      const c = snap.oilTemp > 130 ? '#ef4444' : snap.oilTemp > 115 ? '#facc15' : '#22c55e'
      return r(`${snap.oilTemp.toFixed(1)} °C`, c, Math.min(snap.oilTemp / 150, 1))
    }
    case 'oilPress': {
      const c = snap.oilPress < 2.5 ? '#ef4444' : '#ccc'
      return r(`${snap.oilPress.toFixed(2)} bar`, c)
    }
    case 'fuelPress':
      if (snap.fuelPress == null) return null
      return r(`${snap.fuelPress.toFixed(2)} bar`)
    case 'manifoldPress':
      if (snap.manifoldPress == null) return null
      return r(`${snap.manifoldPress.toFixed(2)} bar`, '#3b82f6')
    case 'voltage': {
      const c = snap.voltage < 12 ? '#ef4444' : snap.voltage < 13 ? '#facc15' : '#22c55e'
      return r(`${snap.voltage.toFixed(1)} V`, c)
    }

    case 'engineWarnings':
      // Rendered specially — pass through with bit value as display
      return r(snap.engineWarnings === 0 ? 'OK' : `0x${snap.engineWarnings.toString(16).toUpperCase()}`,
        snap.engineWarnings !== 0 ? '#ef4444' : '#22c55e')
    case 'shiftLights':
      if (snap.shiftLightShiftRpm == null) return null
      return r(`↑${snap.shiftLightShiftRpm.toFixed(0)}`)
  }
}

function getActiveFields(
  snap: TelemetrySnapshot,
  order: ElectronicsFieldId[],
  hidden: Set<ElectronicsFieldId>,
): ResolvedField[] {
  const result: ResolvedField[] = []
  for (const id of order) {
    if (hidden.has(id)) continue
    const resolved = resolveField(id, snap)
    if (resolved) result.push(resolved)
  }
  return result
}

// ─── Main Widget ──────────────────────────────────────────────────────────────

interface Props {
  snap: TelemetrySnapshot | null
}

export function Electronics({ snap }: Props) {
  const [showSettings, setShowSettings] = useState(false)
  const [order, setOrder] = useState<ElectronicsFieldId[]>(() => loadFields().order)
  const [hidden, setHidden] = useState<Set<ElectronicsFieldId>>(() => loadFields().hidden)
  const [style, setStyle] = useState<ElectronicsStyle>(() => loadPrefs().style)
  const [fontScale, setFontScale] = useState(() => loadPrefs().fontScale)

  function handleOrderChange(o: ElectronicsFieldId[]) {
    setOrder(o)
    saveFields(o, hidden)
  }

  function handleToggle(id: ElectronicsFieldId) {
    const next = new Set(hidden)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    setHidden(next)
    saveFields(order, next)
  }

  function handleStyleChange(s: ElectronicsStyle) {
    setStyle(s)
    savePrefs(s, fontScale)
  }

  function handleFontScale(s: number) {
    setFontScale(s)
    savePrefs(style, s)
  }

  function handleResetAll() {
    setOrder(DEFAULT_FIELD_ORDER)
    setHidden(new Set(DEFAULT_HIDDEN))
    setStyle('bento')
    setFontScale(1)
    saveFields(DEFAULT_FIELD_ORDER, new Set(DEFAULT_HIDDEN))
    savePrefs('bento', 1)
  }

  if (!snap) {
    return (
      <section className="card">
        <h2><Cpu size={14} style={{ verticalAlign: 'middle', marginRight: 5 }} />Electronics</h2>
        <p className="muted">waiting for first frame…</p>
      </section>
    )
  }

  const fields = getActiveFields(snap, order, hidden)

  return (
    <section className="card" style={{ '--widget-font-scale': fontScale } as React.CSSProperties}>
      <h2 style={{ display: 'flex', alignItems: 'center' }}>
        <Cpu size={14} style={{ marginRight: 5, flexShrink: 0 }} />
        Electronics
        <button
          className={`header-btn${showSettings ? ' header-btn-active' : ''}`}
          onClick={() => setShowSettings(v => !v)}
          title="Settings"
          style={{ marginLeft: 'auto' }}
        >⚙</button>
      </h2>

      <div className="card-body">
        {style === 'bento' && <ElectronicsBento snap={snap} fields={fields} fontScale={fontScale} />}
        {style === 'list' && <ElectronicsList snap={snap} fields={fields} fontScale={fontScale} />}
        {style === 'cluster' && <ElectronicsCluster snap={snap} fields={fields} fontScale={fontScale} />}
      </div>

      <SettingsDrawer open={showSettings} onClose={() => setShowSettings(false)} title="Electronics">
        <ElectronicsSettings
          order={order}
          hidden={hidden}
          style={style}
          fontScale={fontScale}
          onOrderChange={handleOrderChange}
          onToggle={handleToggle}
          onStyleChange={handleStyleChange}
          onFontScale={handleFontScale}
          onResetAll={handleResetAll}
        />
      </SettingsDrawer>
    </section>
  )
}
