import {
  DndContext, closestCenter, PointerSensor, useSensor, useSensors, type DragEndEvent,
} from '@dnd-kit/core'
import {
  SortableContext, useSortable, verticalListSortingStrategy, arrayMove,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'

export type ElectronicsFieldId =
  | 'tc1' | 'tc2' | 'abs' | 'bb' | 'throttleShape'
  | 'diffEntry' | 'diffMid' | 'diffExit' | 'arbFront' | 'arbRear' | 'engineBraking'
  | 'mgukMode' | 'mgukPower' | 'battery' | 'drs' | 'p2p'
  | 'gear' | 'rpm' | 'speed' | 'waterTemp' | 'oilTemp' | 'oilPress'
  | 'fuelPress' | 'manifoldPress' | 'voltage'
  | 'engineWarnings' | 'shiftLights'

export type ElectronicsStyle = 'bento' | 'list' | 'cluster'
export type ElectronicsGroup = 'adjustments' | 'hybrid' | 'engine' | 'status'

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

export const FIELD_GROUP: Record<ElectronicsFieldId, ElectronicsGroup> = {
  tc1: 'adjustments', tc2: 'adjustments', abs: 'adjustments', bb: 'adjustments',
  throttleShape: 'adjustments', diffEntry: 'adjustments', diffMid: 'adjustments',
  diffExit: 'adjustments', arbFront: 'adjustments', arbRear: 'adjustments',
  engineBraking: 'adjustments',
  mgukMode: 'hybrid', mgukPower: 'hybrid', battery: 'hybrid', drs: 'hybrid', p2p: 'hybrid',
  gear: 'engine', rpm: 'engine', speed: 'engine', waterTemp: 'engine', oilTemp: 'engine',
  oilPress: 'engine', fuelPress: 'engine', manifoldPress: 'engine', voltage: 'engine',
  engineWarnings: 'status', shiftLights: 'status',
}

const GROUP_LABELS: Record<ElectronicsGroup, string> = {
  adjustments: 'Driver Adjustments',
  hybrid: 'Hybrid / Boost',
  engine: 'Engine',
  status: 'Status',
}

export const DEFAULT_FIELD_ORDER: ElectronicsFieldId[] = [
  'tc1', 'tc2', 'abs', 'bb', 'throttleShape',
  'diffEntry', 'diffMid', 'diffExit', 'arbFront', 'arbRear', 'engineBraking',
  'mgukMode', 'mgukPower', 'battery', 'drs', 'p2p',
  'gear', 'rpm', 'speed', 'waterTemp', 'oilTemp', 'oilPress',
  'fuelPress', 'manifoldPress', 'voltage',
  'engineWarnings', 'shiftLights',
]

export const DEFAULT_HIDDEN: ElectronicsFieldId[] = [
  'throttleShape', 'diffEntry', 'diffMid', 'diffExit',
  'arbFront', 'arbRear', 'engineBraking',
  'fuelPress', 'manifoldPress', 'shiftLights',
]

interface RowProps {
  id: ElectronicsFieldId
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
  order: ElectronicsFieldId[]
  hidden: Set<ElectronicsFieldId>
  style: ElectronicsStyle
  fontScale: number
  onOrderChange: (o: ElectronicsFieldId[]) => void
  onToggle: (id: ElectronicsFieldId) => void
  onStyleChange: (s: ElectronicsStyle) => void
  onFontScale: (s: number) => void
  onResetAll: () => void
}

const STYLES: { id: ElectronicsStyle; label: string }[] = [
  { id: 'bento', label: 'Bento' },
  { id: 'list', label: 'List' },
  { id: 'cluster', label: 'Cluster' },
]

const GROUPS: ElectronicsGroup[] = ['adjustments', 'hybrid', 'engine', 'status']

export function ElectronicsSettings({
  order, hidden, style, fontScale,
  onOrderChange, onToggle, onStyleChange, onFontScale, onResetAll,
}: Props) {
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }))

  function handleDragEnd(e: DragEndEvent) {
    const { active, over } = e
    if (over && active.id !== over.id) {
      const from = order.indexOf(active.id as ElectronicsFieldId)
      const to = order.indexOf(over.id as ElectronicsFieldId)
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
      {/* Style picker */}
      <div className="settings-section">
        <div className="settings-section-title">Layout Style</div>
        <div style={{ display: 'flex', gap: 6 }}>
          {STYLES.map(s => (
            <button
              key={s.id}
              onClick={() => onStyleChange(s.id)}
              className={`settings-btn${style === s.id ? ' settings-btn-active' : ''}`}
              style={{ flex: 1, fontSize: 'var(--settings-fs)' }}
            >
              {s.label}
            </button>
          ))}
        </div>
      </div>

      {/* Field list — grouped by section */}
      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
        <SortableContext items={order} strategy={verticalListSortingStrategy}>
          {groupedOrder.map(({ group, label, ids }) => (
            <div key={group}>
              <div style={{
                padding: '6px 14px 2px',
                fontSize: 'var(--settings-fs-sm)', fontWeight: 600, textTransform: 'uppercase',
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
            <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Font size</label>
            <input
              type="range" min={0.7} max={2.0} step={0.05}
              value={fontScale}
              onChange={e => onFontScale(parseFloat(e.target.value))}
            />
            <span style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>{Math.round(fontScale * 100)}%</span>
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
