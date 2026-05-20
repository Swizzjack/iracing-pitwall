import { useState } from 'react'
import {
  Sun, CloudSun, Cloud, CloudFog, CloudRain,
  Thermometer, Wind, Droplets, Gauge, Eye, Clock, Activity, Info,
} from 'lucide-react'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import type { SessionInfoYaml } from '@shared/SessionInfoYaml'
import { SettingsDrawer } from '../components/SettingsDrawer'
import { WindCompass, windCardinal } from '../components/WindCompass'
import {
  WeatherSettings, DEFAULT_FIELD_ORDER,
  type WeatherFieldId, type TempUnit, type SpeedUnit,
} from './WeatherSettings'

// ─── Unit helpers ─────────────────────────────────────────────────────────────

function fmtTemp(c: number, u: TempUnit) {
  const v = u === 'F' ? c * 9 / 5 + 32 : c
  return `${v.toFixed(1)} °${u}`
}

function fmtWind(ms: number, u: SpeedUnit) {
  return u === 'mph'
    ? `${(ms * 2.23694).toFixed(1)} mph`
    : `${(ms * 3.6).toFixed(1)} km/h`
}

function fmtTime(sec: number) {
  const h = Math.floor(sec / 3600) % 24
  const m = Math.floor((sec % 3600) / 60)
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}`
}

// ─── Wetness config ───────────────────────────────────────────────────────────

const WETNESS_LABELS = [
  '', 'Dry', 'Mostly Dry', 'V. Lightly Wet', 'Lightly Wet',
  'Moderately Wet', 'Very Wet', 'Extremely Wet',
]
const WETNESS_COLORS = [
  '#555', '#22c55e', '#4ade80', '#a3e635', '#facc15',
  '#fb923c', '#3b82f6', '#2563eb',
]

// ─── Rubber state config ──────────────────────────────────────────────────────

const RUBBER_STATES = [
  { label: 'Clean',      color: '#22c55e' },
  { label: 'Slight',     color: '#65a30d' },
  { label: 'Low',        color: '#a3e635' },
  { label: 'Mod. Low',   color: '#d4c000' },
  { label: 'Moderate',   color: '#facc15' },
  { label: 'Mod. High',  color: '#f59e0b' },
  { label: 'High',       color: '#f97316' },
  { label: 'Extensive',  color: '#ef4444' },
  { label: 'Maximum',    color: '#dc2626' },
]

const RUBBER_INDEX: Record<string, number> = {
  'clean':                  0,
  'slight usage':           1,
  'low usage':              2,
  'moderately low usage':   3,
  'moderate usage':         4,
  'moderately high usage':  5,
  'high usage':             6,
  'extensive usage':        7,
  'maximum usage':          8,
}

function rubberStateIndex(state: string): number {
  return RUBBER_INDEX[state.toLowerCase()] ?? -1
}

// ─── Sub-components ───────────────────────────────────────────────────────────

type Icon = React.ComponentType<{ size?: number; color?: string; style?: React.CSSProperties }>

function SkiesRow({ snap }: { snap: TelemetrySnapshot }) {
  const s = snap.skies
  if (s == null) return null
  const icons: Icon[] = [Sun, CloudSun, Cloud, CloudFog]
  const colors = ['#facc15', '#facc15', '#aaa', '#777']
  const labels = ['Clear', 'Partly Cloudy', 'Mostly Cloudy', 'Overcast']
  const i = s >= 0 && s <= 3 ? s : 2
  const SkyIcon = icons[i]
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '8px 0', borderBottom: '1px solid #1a1a1a' }}>
      <SkyIcon size={28} color={colors[i]} />
      <span style={{ color: '#ccc', fontFamily: 'var(--font-heading)', fontSize: 'calc(15px * var(--widget-font-scale, 1))' }}>{labels[i]}</span>
    </div>
  )
}

function DeclaredWetRow({ value }: { value: boolean | null | undefined }) {
  if (value == null) return null
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '6px 0', borderBottom: '1px solid #1a1a1a' }}>
      <CloudRain size={14} color={value ? '#ef4444' : '#22c55e'} />
      <span style={{ color: '#666', fontSize: 'calc(12px * var(--widget-font-scale, 1))', flex: 1 }}>Wet Declared</span>
      <span style={{
        fontSize: 'calc(11px * var(--widget-font-scale, 1))', fontWeight: 700, letterSpacing: '0.06em', padding: '2px 7px',
        borderRadius: 3, background: value ? '#ef444420' : '#22c55e20',
        color: value ? '#ef4444' : '#22c55e',
      }}>
        {value ? 'YES' : 'NO'}
      </span>
    </div>
  )
}

function TempRow({ label, value, unit, color }: { label: string; value: number; unit: TempUnit; color: string }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '5px 0', borderBottom: '1px solid #1a1a1a' }}>
      <Thermometer size={14} color={color} />
      <span style={{ color: '#666', fontSize: 'calc(12px * var(--widget-font-scale, 1))', flex: 1 }}>{label}</span>
      <span style={{ color: '#ccc', fontFamily: 'var(--font-mono)', fontSize: 'calc(13px * var(--widget-font-scale, 1))' }}>{fmtTemp(value, unit)}</span>
    </div>
  )
}

function WetnessBar({ value }: { value: number }) {
  const idx = value - 1
  const color = WETNESS_COLORS[value] ?? '#555'
  return (
    <div style={{ padding: '8px 0', borderBottom: '1px solid #1a1a1a' }}>
      <div style={{ display: 'flex', gap: 2, marginBottom: 5 }}>
        {WETNESS_COLORS.slice(1).map((c, i) => (
          <div
            key={i}
            style={{
              flex: 1, height: 6, borderRadius: 3,
              background: i === idx ? c : '#1f1f1f',
              boxShadow: i === idx ? `0 0 5px ${c}` : 'none',
              transition: 'background 0.3s',
            }}
          />
        ))}
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <span style={{ color: '#555', fontSize: 'calc(11px * var(--widget-font-scale, 1))' }}>Track Wetness</span>
        {idx >= 0 && idx < 7 && (
          <span style={{ color, fontSize: 'calc(11px * var(--widget-font-scale, 1))', fontWeight: 600, marginLeft: 'auto' }}>
            {WETNESS_LABELS[value]}
          </span>
        )}
      </div>
    </div>
  )
}

function RubberStateBar({ value }: { value: string }) {
  const idx = rubberStateIndex(value)
  const isCarryOver = value.toLowerCase() === 'carry over'
  const color = idx >= 0 ? RUBBER_STATES[idx].color : '#94a3b8'
  const label = idx >= 0 ? RUBBER_STATES[idx].label : isCarryOver ? 'C/O' : null
  return (
    <div style={{ padding: '8px 0', borderBottom: '1px solid #1a1a1a' }}>
      <div style={{ display: 'flex', gap: 2, marginBottom: 5 }}>
        {RUBBER_STATES.map((s, i) => (
          <div
            key={i}
            style={{
              flex: 1, height: 6, borderRadius: 3,
              background: i === idx ? s.color : '#1f1f1f',
              boxShadow: i === idx ? `0 0 5px ${s.color}` : 'none',
              transition: 'background 0.3s',
            }}
          />
        ))}
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <span style={{ color: '#555', fontSize: 'calc(11px * var(--widget-font-scale, 1))' }}>Track Rubber</span>
        {label && (
          <span style={{ color, fontSize: 'calc(11px * var(--widget-font-scale, 1))', fontWeight: 600, marginLeft: 'auto' }}>
            {label}
          </span>
        )}
      </div>
    </div>
  )
}

function WindRow({ snap, speedUnit }: { snap: TelemetrySnapshot; speedUnit: SpeedUnit }) {
  const vel = snap.windVel
  const dir = snap.windDir
  if (vel == null && dir == null) return null

  const cardinal = dir != null ? windCardinal(dir) : null

  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '8px 0', borderBottom: '1px solid #1a1a1a' }}>
      {dir != null
        ? <WindCompass windRad={dir} size={40} />
        : <Wind size={28} color="#555" style={{ flexShrink: 0 }} />
      }
      <div>
        <div style={{ color: '#ccc', fontFamily: 'var(--font-heading)', fontSize: 'calc(15px * var(--widget-font-scale, 1))' }}>
          {vel != null ? fmtWind(vel, speedUnit) : '–'}
        </div>
        {cardinal && <div style={{ color: '#555', fontSize: 'calc(11px * var(--widget-font-scale, 1))' }}>{cardinal}</div>}
      </div>
    </div>
  )
}

function PercentRow({ label, value, Icon, color }: { label: string; value: number; Icon: Icon; color: string }) {
  const pct = Math.min(value * 100, 100)
  return (
    <div style={{ padding: '5px 0', borderBottom: '1px solid #1a1a1a' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 3 }}>
        <Icon size={14} color={color} />
        <span style={{ color: '#666', fontSize: 'calc(12px * var(--widget-font-scale, 1))', flex: 1 }}>{label}</span>
        <span style={{ color: '#ccc', fontFamily: 'var(--font-mono)', fontSize: 'calc(12px * var(--widget-font-scale, 1))' }}>{pct.toFixed(0)}%</span>
      </div>
      <div style={{ height: 3, background: '#1a1a1a', borderRadius: 2 }}>
        <div style={{ height: '100%', width: `${pct}%`, background: color, borderRadius: 2 }} />
      </div>
    </div>
  )
}

function KvRow({ label, value, Icon }: { label: string; value: string; Icon?: Icon }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '5px 0', borderBottom: '1px solid #1a1a1a' }}>
      {Icon && <Icon size={14} color="#555" />}
      {!Icon && <span style={{ width: 14 }} />}
      <span style={{ color: '#666', fontSize: 'calc(12px * var(--widget-font-scale, 1))', flex: 1 }}>{label}</span>
      <span style={{ color: '#ccc', fontFamily: 'var(--font-mono)', fontSize: 'calc(12px * var(--widget-font-scale, 1))' }}>{value}</span>
    </div>
  )
}

// ─── Field renderer ───────────────────────────────────────────────────────────

function renderField(
  id: WeatherFieldId,
  snap: TelemetrySnapshot,
  info: SessionInfoYaml | null,
  tempUnit: TempUnit,
  speedUnit: SpeedUnit,
): React.ReactNode | null {
  switch (id) {
    case 'skies':
      return snap.skies != null ? <SkiesRow snap={snap} /> : null

    case 'airTemp':
      return <TempRow label="Air Temp" value={snap.airTemp} unit={tempUnit} color="#60a5fa" />

    case 'trackTemp':
      return <TempRow label="Track Temp" value={snap.trackTemp} unit={tempUnit} color="#f97316" />

    case 'wetness':
      return snap.trackWetness != null && snap.trackWetness > 0
        ? <WetnessBar value={snap.trackWetness} />
        : null

    case 'declaredWet':
      return snap.weatherDeclaredWet != null
        ? <DeclaredWetRow value={snap.weatherDeclaredWet} />
        : null

    case 'precipitation':
      return snap.precipitation != null
        ? <PercentRow label="Precipitation" value={snap.precipitation} Icon={CloudRain} color="#3b82f6" />
        : null

    case 'wind':
      return (snap.windVel != null || snap.windDir != null)
        ? <WindRow snap={snap} speedUnit={speedUnit} />
        : null

    case 'humidity':
      return snap.relativeHumidity != null
        ? <PercentRow label="Humidity" value={snap.relativeHumidity} Icon={Droplets} color="#06b6d4" />
        : null

    case 'fog':
      return snap.fogLevel != null && snap.fogLevel > 0
        ? <PercentRow label="Fog" value={snap.fogLevel} Icon={Eye} color="#94a3b8" />
        : null

    case 'pressure':
      return snap.airPressure != null
        ? <KvRow label="Air Pressure" value={`${(snap.airPressure / 100).toFixed(1)} hPa`} Icon={Gauge} />
        : null

    case 'density':
      return snap.airDensity != null
        ? <KvRow label="Air Density" value={`${snap.airDensity.toFixed(3)} kg/m³`} Icon={Activity} />
        : null

    case 'timeOfDay':
      return snap.sessionTimeOfDay != null
        ? <KvRow label="Time of Day" value={fmtTime(snap.sessionTimeOfDay)} Icon={Clock} />
        : null

    case 'weatherType': {
      const wt = info?.WeekendInfo?.TrackWeatherType
      return wt
        ? <KvRow label="Weather Mode" value={wt} Icon={Info} />
        : null
    }

    case 'rubberState': {
      const session = info?.SessionInfo?.Sessions?.find(
        (s) => s.SessionNum === snap.sessionNum
      )
      const rs = session?.SessionTrackRubberState
      return rs ? <RubberStateBar value={rs} /> : null
    }
  }
}

// ─── localStorage helpers ─────────────────────────────────────────────────────

const FIELDS_KEY = 'iracing-weather-fields-v1'
const UNITS_KEY = 'iracing-weather-units-v1'
const FONTSCALE_KEY = 'iracing-weather-fontscale-v1'

function loadFields(): { order: WeatherFieldId[]; hidden: Set<WeatherFieldId> } {
  try {
    const raw = localStorage.getItem(FIELDS_KEY)
    if (raw) {
      const p = JSON.parse(raw)
      const stored = Array.isArray(p.order) ? p.order as WeatherFieldId[] : DEFAULT_FIELD_ORDER
      // Append fields added after the user last saved (e.g. rubberState)
      const missing = DEFAULT_FIELD_ORDER.filter(id => !stored.includes(id))
      const order = missing.length > 0 ? [...stored, ...missing] : stored
      const hidden = new Set<WeatherFieldId>(Array.isArray(p.hidden) ? p.hidden : [])
      return { order, hidden }
    }
  } catch { /* ignore */ }
  return { order: DEFAULT_FIELD_ORDER, hidden: new Set() }
}

function loadUnits(): { tempUnit: TempUnit; speedUnit: SpeedUnit } {
  try {
    const raw = localStorage.getItem(UNITS_KEY)
    if (raw) {
      const p = JSON.parse(raw)
      return {
        tempUnit: p.temp === 'F' ? 'F' : 'C',
        speedUnit: p.speed === 'mph' ? 'mph' : 'kmh',
      }
    }
  } catch { /* ignore */ }
  return { tempUnit: 'C', speedUnit: 'kmh' }
}

function loadFontScale(): number {
  try {
    const raw = localStorage.getItem(FONTSCALE_KEY)
    if (raw) {
      const v = parseFloat(raw)
      if (!isNaN(v)) return v
    }
  } catch { /* ignore */ }
  return 1
}

function saveFields(order: WeatherFieldId[], hidden: Set<WeatherFieldId>) {
  try { localStorage.setItem(FIELDS_KEY, JSON.stringify({ order, hidden: [...hidden] })) } catch { /* ignore */ }
}

function saveUnits(temp: TempUnit, speed: SpeedUnit) {
  try { localStorage.setItem(UNITS_KEY, JSON.stringify({ temp, speed })) } catch { /* ignore */ }
}

function saveFontScale(scale: number) {
  try { localStorage.setItem(FONTSCALE_KEY, String(scale)) } catch { /* ignore */ }
}

// ─── Main widget ──────────────────────────────────────────────────────────────

interface Props {
  snap: TelemetrySnapshot | null
  info: SessionInfoYaml | null
}

export function Weather({ snap, info }: Props) {
  const [showSettings, setShowSettings] = useState(false)

  const [order, setOrder] = useState<WeatherFieldId[]>(() => loadFields().order)
  const [hidden, setHidden] = useState<Set<WeatherFieldId>>(() => loadFields().hidden)
  const [tempUnit, setTempUnit] = useState<TempUnit>(() => loadUnits().tempUnit)
  const [speedUnit, setSpeedUnit] = useState<SpeedUnit>(() => loadUnits().speedUnit)
  const [fontScale, setFontScale] = useState<number>(loadFontScale)

  function handleOrderChange(o: WeatherFieldId[]) { setOrder(o); saveFields(o, hidden) }

  function handleToggle(id: WeatherFieldId) {
    const next = new Set(hidden)
    if (next.has(id)) next.delete(id); else next.add(id)
    setHidden(next)
    saveFields(order, next)
  }

  function handleTempUnit(u: TempUnit) { setTempUnit(u); saveUnits(u, speedUnit) }
  function handleSpeedUnit(u: SpeedUnit) { setSpeedUnit(u); saveUnits(tempUnit, u) }
  function handleFontScale(scale: number) { setFontScale(scale); saveFontScale(scale) }

  function handleResetAll() {
    const o = DEFAULT_FIELD_ORDER
    const h = new Set<WeatherFieldId>()
    setOrder(o); setHidden(h); setTempUnit('C'); setSpeedUnit('kmh'); setFontScale(1)
    saveFields(o, h); saveUnits('C', 'kmh'); saveFontScale(1)
  }

  if (!snap) {
    return (
      <section className="card">
        <h2>Weather</h2>
        <div className="card-body">
          <p className="muted">waiting for first frame…</p>
        </div>
      </section>
    )
  }

  const visible = order.filter(id => !hidden.has(id))

  return (
    <section className="card" style={{ '--widget-font-scale-local': fontScale } as React.CSSProperties}>
      <h2 style={{ display: 'flex', alignItems: 'center' }}>
        Weather
        <button
          className={`header-btn${showSettings ? ' header-btn-active' : ''}`}
          style={{ marginLeft: 'auto' }}
          onClick={() => setShowSettings(s => !s)}
        >⚙</button>
      </h2>
      <SettingsDrawer open={showSettings} onClose={() => setShowSettings(false)} title="Weather Settings">
        <WeatherSettings
          order={order}
          hidden={hidden}
          tempUnit={tempUnit}
          speedUnit={speedUnit}
          fontScale={fontScale}
          onOrderChange={handleOrderChange}
          onToggle={handleToggle}
          onTempUnit={handleTempUnit}
          onSpeedUnit={handleSpeedUnit}
          onFontScale={handleFontScale}
          onResetAll={handleResetAll}
        />
      </SettingsDrawer>
      <div className="card-body" style={{ padding: '4px 16px 12px', overflowY: 'auto' }}>
        {visible.map(id => {
          const node = renderField(id, snap, info, tempUnit, speedUnit)
          if (node == null) return null
          return <div key={id}>{node}</div>
        })}
      </div>
    </section>
  )
}
