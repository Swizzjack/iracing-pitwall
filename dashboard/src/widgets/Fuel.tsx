import { useState } from 'react'
import {
  Fuel as FuelIcon, Clock, Flag, TrendingDown, BarChart2,
  Timer, Target, ChevronsRight, Info, ParkingCircle,
  Battery, Zap, Activity,
} from 'lucide-react'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import { fmtLapTime, fmtDuration } from '../format'
import { SettingsDrawer } from '../components/SettingsDrawer'
import {
  FuelSettings, DEFAULT_FIELD_ORDER, DEFAULT_HIDDEN,
  type FuelFieldId, type LapWindow, type ReserveUnit,
} from './FuelSettings'
import { useFuelTracker } from './useFuelTracker'

// ─── localStorage ─────────────────────────────────────────────────────────────

const FIELDS_KEY = 'iracing-fuel-fields-v1'
const PREFS_KEY = 'iracing-fuel-prefs-v1'

function loadFields(): { order: FuelFieldId[]; hidden: Set<FuelFieldId> } {
  try {
    const raw = localStorage.getItem(FIELDS_KEY)
    if (raw) {
      const p = JSON.parse(raw)
      const order = Array.isArray(p.order) ? p.order as FuelFieldId[] : DEFAULT_FIELD_ORDER
      const hidden = new Set<FuelFieldId>(Array.isArray(p.hidden) ? p.hidden : DEFAULT_HIDDEN)
      return { order, hidden }
    }
  } catch { /* ignore */ }
  return { order: DEFAULT_FIELD_ORDER, hidden: new Set(DEFAULT_HIDDEN) }
}

function loadPrefs(): { lapWindow: LapWindow; reserveValue: number; reserveUnit: ReserveUnit; fontScale: number } {
  try {
    const raw = localStorage.getItem(PREFS_KEY)
    if (raw) {
      const p = JSON.parse(raw)
      const lapWindow: LapWindow = ([3, 5, 'all'] as LapWindow[]).includes(p.lapWindow)
        ? p.lapWindow : 5
      return {
        lapWindow,
        reserveValue: typeof p.reserveValue === 'number' ? p.reserveValue : 0,
        reserveUnit: p.reserveUnit === 'laps' ? 'laps' : 'L',
        fontScale: typeof p.fontScale === 'number' ? p.fontScale : 1,
      }
    }
  } catch { /* ignore */ }
  return { lapWindow: 5, reserveValue: 0, reserveUnit: 'L', fontScale: 1 }
}

function saveFields(order: FuelFieldId[], hidden: Set<FuelFieldId>) {
  try { localStorage.setItem(FIELDS_KEY, JSON.stringify({ order, hidden: [...hidden] })) } catch { /* ignore */ }
}

function savePrefs(lapWindow: LapWindow, reserveValue: number, reserveUnit: ReserveUnit, fontScale: number) {
  try { localStorage.setItem(PREFS_KEY, JSON.stringify({ lapWindow, reserveValue, reserveUnit, fontScale })) } catch { /* ignore */ }
}

// ─── MGU-K Mode labels ────────────────────────────────────────────────────────

const MGUK_MODES: Record<number, string> = {
  0: 'Off', 1: 'Low', 2: 'Medium', 3: 'High',
  4: 'Hot Lap', 5: 'Time Attack',
}

// ─── Row components ───────────────────────────────────────────────────────────

type Icon = React.ComponentType<{ size?: number; color?: string }>

function KvRow({ label, value, icon: Icon, valueColor }: {
  label: string; value: string; icon?: Icon; valueColor?: string
}) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '5px 0', borderBottom: '1px solid #1a1a1a' }}>
      {Icon
        ? <Icon size={14} color="#555" />
        : <span style={{ width: 14 }} />}
      <span style={{ color: '#666', fontSize: 'calc(12px * var(--widget-font-scale, 1))', flex: 1 }}>{label}</span>
      <span style={{
        color: valueColor ?? '#ccc',
        fontFamily: 'var(--font-mono)',
        fontSize: 'calc(13px * var(--widget-font-scale, 1))',
      }}>{value}</span>
    </div>
  )
}

function LevelRow({ level, pct }: { level: number; pct: number }) {
  const filled = Math.min(pct, 1)
  const color = pct > 0.3 ? '#22c55e' : pct > 0.15 ? '#facc15' : '#ef4444'
  return (
    <div style={{ padding: '6px 0', borderBottom: '1px solid #1a1a1a' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
        <FuelIcon size={14} color={color} />
        <span style={{ color: '#666', fontSize: 'calc(12px * var(--widget-font-scale, 1))', flex: 1 }}>Fuel level</span>
        <span style={{ color: '#ccc', fontFamily: 'var(--font-mono)', fontSize: 'calc(13px * var(--widget-font-scale, 1))' }}>
          {level.toFixed(2)} L <span style={{ color: '#555' }}>({(pct * 100).toFixed(0)}%)</span>
        </span>
      </div>
      <div style={{ height: 4, background: '#1a1a1a', borderRadius: 2 }}>
        <div style={{ height: '100%', width: `${filled * 100}%`, background: color, borderRadius: 2, transition: 'width 0.4s' }} />
      </div>
    </div>
  )
}

function SurplusRow({ surplus, label }: { surplus: number; label: string }) {
  const positive = surplus >= 0
  const color = positive ? '#22c55e' : '#ef4444'
  const sign = positive ? '+' : ''
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '5px 0', borderBottom: '1px solid #1a1a1a' }}>
      <ChevronsRight size={14} color={color} />
      <span style={{ color: '#666', fontSize: 'calc(12px * var(--widget-font-scale, 1))', flex: 1 }}>{label}</span>
      <span style={{
        color,
        fontFamily: 'var(--font-mono)',
        fontSize: 'calc(13px * var(--widget-font-scale, 1))',
        fontWeight: 600,
      }}>{sign}{surplus.toFixed(2)} L</span>
    </div>
  )
}

function BatteryRow({ pct }: { pct: number }) {
  const color = pct > 0.3 ? '#3b82f6' : pct > 0.15 ? '#facc15' : '#ef4444'
  return (
    <div style={{ padding: '6px 0', borderBottom: '1px solid #1a1a1a' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
        <Battery size={14} color={color} />
        <span style={{ color: '#666', fontSize: 'calc(12px * var(--widget-font-scale, 1))', flex: 1 }}>Battery (ERS)</span>
        <span style={{ color: '#ccc', fontFamily: 'var(--font-mono)', fontSize: 'calc(13px * var(--widget-font-scale, 1))' }}>
          {(pct * 100).toFixed(0)}%
        </span>
      </div>
      <div style={{ height: 4, background: '#1a1a1a', borderRadius: 2 }}>
        <div style={{ height: '100%', width: `${Math.min(pct, 1) * 100}%`, background: color, borderRadius: 2, transition: 'width 0.4s' }} />
      </div>
    </div>
  )
}

// ─── Field renderer ───────────────────────────────────────────────────────────

interface FieldCtx {
  snap: TelemetrySnapshot
  avgPerLap: number | null
  lastLapFuel: number | null
  worstLap: number | null
  lapWindow: LapWindow
  timeLeftS: number | null
  lapsLeftInTank: number | null
  needToFinish: number | null
  surplus: number | null
  isTimeBased: boolean
  fuelAtLapStart: number | null
}

function renderField(id: FuelFieldId, ctx: FieldCtx): React.ReactNode | null {
  const { snap, avgPerLap, lastLapFuel, worstLap, lapWindow,
    timeLeftS, lapsLeftInTank, needToFinish, surplus, isTimeBased,
    fuelAtLapStart } = ctx

  switch (id) {
    case 'level':
      return <LevelRow level={snap.fuelLevel} pct={snap.fuelLevelPct} />

    case 'timeLeft':
      return <KvRow label="Time left (tank)" value={timeLeftS != null ? fmtDuration(timeLeftS) : '—'} icon={Clock} />

    case 'lapsLeft':
      return <KvRow label="Laps left (tank)" value={lapsLeftInTank != null ? lapsLeftInTank.toFixed(1) : '—'} icon={Flag} />

    case 'perLap': {
      const windowLabel = lapWindow === 'all' ? 'all' : `${lapWindow}`
      const label = `Per lap (avg ${windowLabel})`
      return <KvRow label={label} value={avgPerLap != null ? `${avgPerLap.toFixed(2)} L` : '—'} icon={BarChart2} />
    }

    case 'lastLap':
      return <KvRow label="Last lap usage" value={lastLapFuel != null ? `${lastLapFuel.toFixed(2)} L` : '—'} icon={TrendingDown} />

    case 'worstLap':
      return <KvRow label="Worst lap" value={worstLap != null ? `${worstLap.toFixed(2)} L` : '—'} icon={TrendingDown} />

    case 'perHour':
      return <KvRow label="Per hour" value={snap.fuelUsePerHour > 0 ? `${snap.fuelUsePerHour.toFixed(2)} L/h` : '—'} icon={Activity} />

    case 'lastLapTime':
      return <KvRow label="Last lap time" value={fmtLapTime(snap.lapLastTime)} icon={Timer} />

    case 'needToFinish':
      return <KvRow
        label="Need to finish"
        value={needToFinish != null ? `${needToFinish.toFixed(2)} L` : '—'}
        icon={Target}
      />

    case 'surplus':
      return surplus != null ? <SurplusRow surplus={surplus} label="Surplus / Deficit" /> : null

    case 'format': {
      let value: string
      if (isTimeBased) {
        const remain = snap.sessionTimeRemain
        value = remain != null && remain > 0 ? `Time · ${fmtDuration(remain)} left` : 'Time race'
      } else {
        const lr = snap.sessionLapsRemain
        value = lr != null ? `Laps · ${lr} left` : 'Lap race'
      }
      return <KvRow label="Race format" value={value} icon={Info} />
    }

    case 'pitStops': {
      const refFuel = fuelAtLapStart ?? snap.fuelLevel
      if (needToFinish == null || needToFinish <= refFuel) {
        return <KvRow label="Stops to finish" value="0" icon={ParkingCircle} />
      }
      const stops = Math.ceil((needToFinish - refFuel) / Math.max(refFuel, 1))
      return <KvRow label="Stops to finish" value={String(stops)} icon={ParkingCircle} />
    }

    case 'batteryPct':
      return snap.energyBatteryPct != null ? <BatteryRow pct={snap.energyBatteryPct} /> : null

    case 'mgukMode':
      return snap.mgukDeployMode != null
        ? <KvRow label="MGU-K Mode" value={MGUK_MODES[snap.mgukDeployMode] ?? `Mode ${snap.mgukDeployMode}`} icon={Zap} />
        : null

    case 'mgukPower':
      return snap.powerMguk != null && snap.powerMguk > 0
        ? <KvRow label="MGU-K Power" value={`${(snap.powerMguk / 1000).toFixed(1)} kW`} icon={Zap} />
        : null
  }
}

// ─── Main widget ──────────────────────────────────────────────────────────────

interface Props {
  snap: TelemetrySnapshot | null
}

export function Fuel({ snap }: Props) {
  const [showSettings, setShowSettings] = useState(false)
  const [order, setOrder] = useState<FuelFieldId[]>(() => loadFields().order)
  const [hidden, setHidden] = useState<Set<FuelFieldId>>(() => loadFields().hidden)
  const prefs = useState(() => loadPrefs())[0]
  const [lapWindow, setLapWindow] = useState<LapWindow>(prefs.lapWindow)
  const [reserveValue, setReserveValue] = useState(prefs.reserveValue)
  const [reserveUnit, setReserveUnit] = useState<ReserveUnit>(prefs.reserveUnit)
  const [fontScale, setFontScale] = useState(prefs.fontScale)

  const tracker = useFuelTracker(snap, lapWindow)
  const {
    avgPerLap, lastLapFuel, worstLap,
    fuelAtLapStart,
    lapLastTimeAtLapStart,
    sessionTimeRemainAtLapStart,
    sessionLapsRemainAtLapStart,
    fuelUsePerHourAtLapStart,
  } = tracker

  function handleOrderChange(o: FuelFieldId[]) { setOrder(o); saveFields(o, hidden) }
  function handleToggle(id: FuelFieldId) {
    const next = new Set(hidden)
    if (next.has(id)) next.delete(id); else next.add(id)
    setHidden(next); saveFields(order, next)
  }
  function handleLapWindow(w: LapWindow) { setLapWindow(w); savePrefs(w, reserveValue, reserveUnit, fontScale) }
  function handleReserveValue(v: number) { setReserveValue(v); savePrefs(lapWindow, v, reserveUnit, fontScale) }
  function handleReserveUnit(u: ReserveUnit) { setReserveUnit(u); savePrefs(lapWindow, reserveValue, u, fontScale) }
  function handleFontScale(s: number) { setFontScale(s); savePrefs(lapWindow, reserveValue, reserveUnit, s) }
  function handleResetTracker() { tracker.reset() }
  function handleResetAll() {
    const o = DEFAULT_FIELD_ORDER
    const h = new Set<FuelFieldId>(DEFAULT_HIDDEN)
    setOrder(o); setHidden(h); setLapWindow(5); setReserveValue(0); setReserveUnit('L'); setFontScale(1)
    saveFields(o, h); savePrefs(5, 0, 'L', 1)
    tracker.reset()
  }

  if (!snap) {
    return (
      <section className="card">
        <h2>Fuel</h2>
        <div className="card-body">
          <p className="muted">waiting for first frame…</p>
        </div>
      </section>
    )
  }

  // ── Derived predictions ──────────────────────────────────────────────────────

  // Fallback avg from snapshot values so it stays stable between laps
  const effectiveAvg = avgPerLap ?? (
    fuelUsePerHourAtLapStart != null && lapLastTimeAtLapStart != null
      ? (fuelUsePerHourAtLapStart * lapLastTimeAtLapStart) / 3600
      : null
  )

  // Live: decrease continuously like a clock (intentional — it's a countdown)
  const lapsLeftInTank = effectiveAvg != null && effectiveAvg > 0
    ? snap.fuelLevel / effectiveAvg : null
  const timeLeftS = lapsLeftInTank != null && snap.lapLastTime > 0
    ? lapsLeftInTank * snap.lapLastTime : null

  // Race format detection: 32767 is iRacing's sentinel for "unlimited laps" (time-based)
  const snapshotLapsRemain = sessionLapsRemainAtLapStart
  const isTimeBased = snapshotLapsRemain == null || snapshotLapsRemain >= 32767

  // Per-lap stable: use snapshot inputs so lapsToFinish doesn't step-jump every second
  let lapsToFinish: number | null = null
  if (isTimeBased) {
    if (sessionTimeRemainAtLapStart != null && sessionTimeRemainAtLapStart > 0
        && lapLastTimeAtLapStart != null && lapLastTimeAtLapStart > 0) {
      lapsToFinish = Math.ceil(sessionTimeRemainAtLapStart / lapLastTimeAtLapStart) + 1
    }
  } else {
    lapsToFinish = snapshotLapsRemain
  }

  const reserveL = reserveUnit === 'L'
    ? reserveValue
    : (effectiveAvg != null ? reserveValue * effectiveAvg : 0)

  const needToFinish = lapsToFinish != null && effectiveAvg != null
    ? lapsToFinish * effectiveAvg + reserveL : null

  // Surplus vs lap-start fuel snapshot so it stays constant between lap crossings
  const surplus = needToFinish != null && fuelAtLapStart != null
    ? fuelAtLapStart - needToFinish : null

  const ctx: FieldCtx = {
    snap, avgPerLap: effectiveAvg, lastLapFuel, worstLap, lapWindow,
    timeLeftS, lapsLeftInTank, needToFinish, surplus, isTimeBased, fuelAtLapStart,
  }

  const visible = order.filter(id => !hidden.has(id))

  return (
    <section className="card" style={{ '--widget-font-scale': fontScale } as React.CSSProperties}>
      <h2 style={{ display: 'flex', alignItems: 'center' }}>
        Fuel
        <button
          className={`header-btn${showSettings ? ' header-btn-active' : ''}`}
          style={{ marginLeft: 'auto' }}
          onClick={() => setShowSettings(s => !s)}
        >⚙</button>
      </h2>
      <SettingsDrawer open={showSettings} onClose={() => setShowSettings(false)} title="Fuel Settings">
        <FuelSettings
          order={order}
          hidden={hidden}
          lapWindow={lapWindow}
          reserveValue={reserveValue}
          reserveUnit={reserveUnit}
          fontScale={fontScale}
          onOrderChange={handleOrderChange}
          onToggle={handleToggle}
          onLapWindow={handleLapWindow}
          onReserveValue={handleReserveValue}
          onReserveUnit={handleReserveUnit}
          onFontScale={handleFontScale}
          onResetTracker={handleResetTracker}
          onResetAll={handleResetAll}
        />
      </SettingsDrawer>
      <div className="card-body" style={{ padding: '4px 16px 12px', overflowY: 'auto' }}>
        {visible.map(id => {
          const node = renderField(id, ctx)
          if (node == null) return null
          return <div key={id}>{node}</div>
        })}
      </div>
    </section>
  )
}
