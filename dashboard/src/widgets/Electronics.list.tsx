import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import {
  Gauge, Zap, Thermometer, Droplets, Activity,
  ToggleLeft, Wind, AlertTriangle, Cpu,
} from 'lucide-react'
import type { ResolvedField } from './Electronics'

interface Props {
  snap: TelemetrySnapshot
  fields: ResolvedField[]
  fontScale: number
}

type IconComp = React.ComponentType<{ size?: number; color?: string }>

const FIELD_ICONS: Partial<Record<string, IconComp>> = {
  tc1: ToggleLeft, tc2: ToggleLeft, abs: ToggleLeft, bb: Gauge,
  throttleShape: Gauge, diffEntry: Cpu, diffMid: Cpu, diffExit: Cpu,
  arbFront: Wind, arbRear: Wind, engineBraking: Gauge,
  mgukMode: Zap, mgukPower: Zap, battery: Zap,
  drs: Wind, p2p: Zap,
  gear: Gauge, rpm: Gauge, speed: Gauge,
  waterTemp: Thermometer, oilTemp: Thermometer, oilPress: Droplets,
  fuelPress: Droplets, manifoldPress: Activity, voltage: Zap,
  engineWarnings: AlertTriangle, shiftLights: Activity,
}

const GROUP_LABELS: Record<string, string> = {
  adjustments: 'Driver Adjustments',
  hybrid: 'Hybrid / Boost',
  engine: 'Engine',
  status: 'Status',
}

function KvRow({ field, fontScale }: { field: ResolvedField; fontScale: number }) {
  const Icon = FIELD_ICONS[field.id]
  const fs = `calc(${13 * fontScale}px)`
  const labelFs = `calc(${12 * fontScale}px)`
  return (
    <div style={{
      display: 'flex', alignItems: 'center', gap: 8,
      padding: '5px 0', borderBottom: '1px solid #1a1a1a',
    }}>
      {Icon
        ? <Icon size={14} color={field.color ?? '#555'} />
        : <span style={{ width: 14 }} />}
      <span style={{ color: '#666', fontSize: labelFs, flex: 1 }}>{field.label}</span>
      <span style={{ color: field.color ?? '#ccc', fontFamily: 'var(--font-mono)', fontSize: fs }}>
        {field.value}
      </span>
    </div>
  )
}

function BarRow({ field, fontScale }: { field: ResolvedField; fontScale: number }) {
  const Icon = FIELD_ICONS[field.id] ?? Thermometer
  const fs = `calc(${13 * fontScale}px)`
  const labelFs = `calc(${12 * fontScale}px)`
  const pct = field.barPct ?? 0
  return (
    <div style={{ padding: '5px 0', borderBottom: '1px solid #1a1a1a' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 3 }}>
        <Icon size={14} color={field.color ?? '#555'} />
        <span style={{ color: '#666', fontSize: labelFs, flex: 1 }}>{field.label}</span>
        <span style={{ color: field.color ?? '#ccc', fontFamily: 'var(--font-mono)', fontSize: fs }}>
          {field.value}
        </span>
      </div>
      <div style={{ height: 3, background: '#1a1a1a', borderRadius: 2 }}>
        <div style={{
          height: '100%', width: `${Math.min(pct, 1) * 100}%`,
          background: field.color ?? '#3b82f6', borderRadius: 2, transition: 'width 0.3s',
        }} />
      </div>
    </div>
  )
}

function WarningLeds({ warnings, fontScale }: { warnings: number; fontScale: number }) {
  const bits = [
    { bit: 0x01, label: 'H₂O', warn: true },
    { bit: 0x02, label: 'FUEL', warn: true },
    { bit: 0x04, label: 'OIL-P', warn: true },
    { bit: 0x08, label: 'STALL', warn: true },
    { bit: 0x10, label: 'PIT', warn: false },
    { bit: 0x20, label: 'REV', warn: false },
    { bit: 0x40, label: 'OIL-T', warn: true },
  ]
  return (
    <div style={{ padding: '6px 0', borderBottom: '1px solid #1a1a1a' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 5 }}>
        <AlertTriangle size={14} color="#555" />
        <span style={{ color: '#666', fontSize: `calc(${12 * fontScale}px)` }}>Engine Warnings</span>
      </div>
      <div style={{ display: 'flex', gap: 5, flexWrap: 'wrap' }}>
        {bits.map(({ bit, label, warn }) => {
          const active = (warnings & bit) !== 0
          return (
            <span key={bit} style={{
              fontSize: `calc(${10 * fontScale}px)`,
              fontFamily: 'var(--font-mono)',
              padding: '2px 5px', borderRadius: 3,
              background: active ? (warn ? '#ef444420' : '#f9731620') : '#111',
              color: active ? (warn ? '#ef4444' : '#f97316') : '#333',
              border: `1px solid ${active ? (warn ? '#ef444440' : '#f9731640') : '#222'}`,
              transition: 'all 0.2s',
            }}>{label}</span>
          )
        })}
      </div>
    </div>
  )
}

function ShiftRow({ snap, fontScale }: { snap: TelemetrySnapshot; fontScale: number }) {
  const fs = `calc(${12 * fontScale}px)`
  return (
    <div style={{ padding: '5px 0', borderBottom: '1px solid #1a1a1a' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 3 }}>
        <Activity size={14} color="#555" />
        <span style={{ color: '#666', fontSize: fs, flex: 1 }}>Shift Lights</span>
      </div>
      <div style={{ display: 'flex', gap: 12 }}>
        {snap.shiftLightFirstRpm != null && (
          <span style={{ color: '#666', fontSize: fs, fontFamily: 'var(--font-mono)' }}>
            <span style={{ color: '#444' }}>first </span>{(snap.shiftLightFirstRpm).toFixed(0)}
          </span>
        )}
        {snap.shiftLightShiftRpm != null && (
          <span style={{ color: '#facc15', fontSize: fs, fontFamily: 'var(--font-mono)' }}>
            <span style={{ color: '#444' }}>shift </span>{(snap.shiftLightShiftRpm).toFixed(0)}
          </span>
        )}
        {snap.shiftLightBlinkRpm != null && (
          <span style={{ color: '#ef4444', fontSize: fs, fontFamily: 'var(--font-mono)' }}>
            <span style={{ color: '#444' }}>blink </span>{(snap.shiftLightBlinkRpm).toFixed(0)}
          </span>
        )}
      </div>
    </div>
  )
}

export function ElectronicsList({ snap, fields, fontScale }: Props) {
  const groups = ['adjustments', 'hybrid', 'engine', 'status'] as const
  const byGroup = Object.fromEntries(
    groups.map(g => [g, fields.filter(f => f.group === g)])
  ) as Record<string, ResolvedField[]>

  return (
    <div style={{ '--widget-font-scale': fontScale } as React.CSSProperties}>
      {groups.map(group => {
        const gFields = byGroup[group]
        if (gFields.length === 0) return null
        return (
          <div key={group}>
            <div style={{
              padding: '6px 0 3px',
              fontSize: `calc(${10 * fontScale}px)`,
              fontWeight: 600, textTransform: 'uppercase',
              letterSpacing: '0.08em', color: '#3a3a3a',
            }}>{GROUP_LABELS[group]}</div>
            {gFields.map(f => {
              if (f.id === 'engineWarnings') {
                return <WarningLeds key={f.id} warnings={snap.engineWarnings} fontScale={fontScale} />
              }
              if (f.id === 'shiftLights') {
                return <ShiftRow key={f.id} snap={snap} fontScale={fontScale} />
              }
              if (f.barPct !== undefined) {
                return <BarRow key={f.id} field={f} fontScale={fontScale} />
              }
              return <KvRow key={f.id} field={f} fontScale={fontScale} />
            })}
          </div>
        )
      })}
    </div>
  )
}
