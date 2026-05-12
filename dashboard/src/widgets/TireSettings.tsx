import {
  DndContext, closestCenter, PointerSensor, useSensor, useSensors, type DragEndEvent,
} from '@dnd-kit/core'
import {
  SortableContext, useSortable, verticalListSortingStrategy, arrayMove,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'

export type TireFieldId =
  | 'tempSurface' | 'tempCarcass'
  | 'pressureCold' | 'pressureHot' | 'pressureDelta'
  | 'wear'
  | 'wheelSpeed' | 'rideHeight'

export type TireStyle = 'bento' | 'list' | 'cluster'
export type TireGroup = 'temps' | 'pressures' | 'wear' | 'chassis'
export type TempUnit = 'C' | 'F'
export type PressUnit = 'psi' | 'bar' | 'kPa'

const FIELD_LABELS: Record<TireFieldId, string> = {
  tempSurface: 'Surface Temp',
  tempCarcass: 'Carcass Temp',
  pressureCold: 'Cold Pressure',
  pressureHot: 'Hot Pressure',
  pressureDelta: 'Pressure Δ',
  wear: 'Wear (L/M/R)',
  wheelSpeed: 'Wheel Speed',
  rideHeight: 'Ride Height',
}

export const FIELD_GROUP: Record<TireFieldId, TireGroup> = {
  tempSurface: 'temps',
  tempCarcass: 'temps',
  pressureCold: 'pressures',
  pressureHot: 'pressures',
  pressureDelta: 'pressures',
  wear: 'wear',
  wheelSpeed: 'chassis',
  rideHeight: 'chassis',
}

const GROUP_LABELS: Record<TireGroup, string> = {
  temps: 'Temperatures',
  pressures: 'Pressures',
  wear: 'Wear',
  chassis: 'Chassis',
}

export const DEFAULT_FIELD_ORDER: TireFieldId[] = [
  'tempSurface', 'tempCarcass',
  'pressureCold', 'pressureHot', 'pressureDelta',
  'wear',
  'wheelSpeed', 'rideHeight',
]

export const DEFAULT_HIDDEN: TireFieldId[] = [
  'pressureCold', 'pressureDelta', 'wheelSpeed',
]

const GROUPS: TireGroup[] = ['temps', 'pressures', 'wear', 'chassis']

const STYLES: { id: TireStyle; label: string }[] = [
  { id: 'bento', label: 'Bento' },
  { id: 'list', label: 'List' },
  { id: 'cluster', label: 'Cluster' },
]

const TEMP_UNITS: { id: TempUnit; label: string }[] = [
  { id: 'C', label: '°C' },
  { id: 'F', label: '°F' },
]

const PRESS_UNITS: { id: PressUnit; label: string }[] = [
  { id: 'psi', label: 'psi' },
  { id: 'bar', label: 'bar' },
  { id: 'kPa', label: 'kPa' },
]

interface RowProps {
  id: TireFieldId
  visible: boolean
  onToggle: () => void
}

function SortableRow({ id, visible, onToggle }: RowProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id })
  return (
    <div
      ref={setNodeRef}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      className={`settings-col-row${isDragging ? ' dragging-row' : ''}`}
    >
      <span className="drag-handle" {...attributes} {...listeners}>⠿</span>
      <label className="toggle">
        <input type="checkbox" checked={visible} onChange={onToggle} />
        <span className="toggle-track" />
      </label>
      <span style={{ color: visible ? '#ccc' : '#555' }}>{FIELD_LABELS[id]}</span>
    </div>
  )
}

interface Props {
  order: TireFieldId[]
  hidden: Set<TireFieldId>
  style: TireStyle
  fontScale: number
  tempUnit: TempUnit
  pressUnit: PressUnit
  onOrderChange: (o: TireFieldId[]) => void
  onToggle: (id: TireFieldId) => void
  onStyleChange: (s: TireStyle) => void
  onFontScale: (s: number) => void
  onTempUnit: (u: TempUnit) => void
  onPressUnit: (u: PressUnit) => void
  onResetAll: () => void
}

export function TireSettings({
  order, hidden, style, fontScale, tempUnit, pressUnit,
  onOrderChange, onToggle, onStyleChange, onFontScale, onTempUnit, onPressUnit, onResetAll,
}: Props) {
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }))

  function handleDragEnd(e: DragEndEvent) {
    const { active, over } = e
    if (over && active.id !== over.id) {
      const from = order.indexOf(active.id as TireFieldId)
      const to = order.indexOf(over.id as TireFieldId)
      onOrderChange(arrayMove(order, from, to))
    }
  }

  const groupedOrder = GROUPS.map(g => ({
    group: g,
    label: GROUP_LABELS[g],
    ids: order.filter(id => FIELD_GROUP[id] === g),
  }))

  return (
    <>
      <div className="settings-section">
        <div className="settings-section-title">Layout Style</div>
        <div style={{ display: 'flex', gap: 6 }}>
          {STYLES.map(s => (
            <button
              key={s.id}
              onClick={() => onStyleChange(s.id)}
              className={`settings-btn${style === s.id ? ' settings-btn-active' : ''}`}
              style={{ flex: 1, fontSize: 12 }}
            >
              {s.label}
            </button>
          ))}
        </div>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">Units</div>
        <div style={{ display: 'flex', gap: 6, marginBottom: 6 }}>
          {TEMP_UNITS.map(u => (
            <button
              key={u.id}
              onClick={() => onTempUnit(u.id)}
              className={`settings-btn${tempUnit === u.id ? ' settings-btn-active' : ''}`}
              style={{ flex: 1, fontSize: 12 }}
            >
              {u.label}
            </button>
          ))}
        </div>
        <div style={{ display: 'flex', gap: 6 }}>
          {PRESS_UNITS.map(u => (
            <button
              key={u.id}
              onClick={() => onPressUnit(u.id)}
              className={`settings-btn${pressUnit === u.id ? ' settings-btn-active' : ''}`}
              style={{ flex: 1, fontSize: 12 }}
            >
              {u.label}
            </button>
          ))}
        </div>
      </div>

      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
        <SortableContext items={order} strategy={verticalListSortingStrategy}>
          {groupedOrder.map(({ group, label, ids }) => (
            <div key={group}>
              <div style={{
                padding: '6px 14px 2px',
                fontSize: 10, fontWeight: 600, textTransform: 'uppercase',
                letterSpacing: '0.08em', color: '#444',
              }}>{label}</div>
              <div className="settings-col-list" style={{ paddingTop: 0 }}>
                {ids.map(id => (
                  <SortableRow key={id} id={id} visible={!hidden.has(id)} onToggle={() => onToggle(id)} />
                ))}
              </div>
            </div>
          ))}
        </SortableContext>
      </DndContext>

      <div className="settings-drawer-footer">
        <div className="settings-section">
          <div className="settings-section-title">Display</div>
          <div className="settings-footer-row">
            <label style={{ color: '#888', fontSize: 12 }}>Font size</label>
            <input
              type="range" min={0.7} max={2.0} step={0.05}
              value={fontScale}
              onChange={e => onFontScale(parseFloat(e.target.value))}
            />
            <span style={{ color: '#888', fontSize: 12 }}>{Math.round(fontScale * 100)}%</span>
          </div>
        </div>
        <div className="settings-footer-btns">
          <button className="settings-btn" onClick={onResetAll} style={{ flex: 1 }}>
            Reset all
          </button>
        </div>
      </div>
    </>
  )
}
