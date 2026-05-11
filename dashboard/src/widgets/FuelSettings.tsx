import {
  DndContext, closestCenter, PointerSensor, useSensor, useSensors, type DragEndEvent,
} from '@dnd-kit/core'
import {
  SortableContext, useSortable, verticalListSortingStrategy, arrayMove,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'

export type FuelFieldId =
  | 'level' | 'timeLeft' | 'lapsLeft'
  | 'perLap' | 'lastLap' | 'worstLap' | 'perHour' | 'lastLapTime'
  | 'needToFinish' | 'surplus' | 'format' | 'pitStops'
  | 'batteryPct' | 'mgukMode' | 'mgukPower'

export type LapWindow = 3 | 5 | 'all'
export type ReserveUnit = 'L' | 'laps'

const FIELD_LABELS: Record<FuelFieldId, string> = {
  level: 'Fuel Level',
  timeLeft: 'Time Left (tank)',
  lapsLeft: 'Laps Left (tank)',
  perLap: 'Per Lap (avg)',
  lastLap: 'Last Lap Usage',
  worstLap: 'Worst Lap',
  perHour: 'Per Hour',
  lastLapTime: 'Last Lap Time',
  needToFinish: 'Need to Finish',
  surplus: 'Surplus / Deficit',
  format: 'Race Format',
  pitStops: 'Stops to Finish',
  batteryPct: 'Battery (ERS)',
  mgukMode: 'MGU-K Mode',
  mgukPower: 'MGU-K Power',
}

export const DEFAULT_FIELD_ORDER: FuelFieldId[] = [
  'level', 'timeLeft', 'lapsLeft',
  'perLap', 'lastLap', 'worstLap', 'perHour', 'lastLapTime',
  'needToFinish', 'surplus', 'format', 'pitStops',
  'batteryPct', 'mgukMode', 'mgukPower',
]

export const DEFAULT_HIDDEN: FuelFieldId[] = ['batteryPct', 'mgukMode', 'mgukPower']

interface RowProps {
  id: FuelFieldId
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
  order: FuelFieldId[]
  hidden: Set<FuelFieldId>
  lapWindow: LapWindow
  reserveValue: number
  reserveUnit: ReserveUnit
  fontScale: number
  onOrderChange: (o: FuelFieldId[]) => void
  onToggle: (id: FuelFieldId) => void
  onLapWindow: (w: LapWindow) => void
  onReserveValue: (v: number) => void
  onReserveUnit: (u: ReserveUnit) => void
  onFontScale: (s: number) => void
  onResetTracker: () => void
  onResetAll: () => void
}

export function FuelSettings({
  order, hidden, lapWindow, reserveValue, reserveUnit, fontScale,
  onOrderChange, onToggle, onLapWindow, onReserveValue, onReserveUnit,
  onFontScale, onResetTracker, onResetAll,
}: Props) {
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }))

  function handleDragEnd(e: DragEndEvent) {
    const { active, over } = e
    if (over && active.id !== over.id) {
      const from = order.indexOf(active.id as FuelFieldId)
      const to = order.indexOf(over.id as FuelFieldId)
      onOrderChange(arrayMove(order, from, to))
    }
  }

  const radioLabel: React.CSSProperties = {
    display: 'flex', alignItems: 'center', gap: 5,
    cursor: 'pointer', color: '#ccc', fontSize: 12,
  }

  return (
    <>
      <div className="settings-col-list">
        <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
          <SortableContext items={order} strategy={verticalListSortingStrategy}>
            {order.map(id => (
              <SortableRow key={id} id={id} visible={!hidden.has(id)} onToggle={() => onToggle(id)} />
            ))}
          </SortableContext>
        </DndContext>
      </div>

      <div className="settings-drawer-footer">
        <div className="settings-section">
          <div className="settings-section-title">Lap Window</div>
          <div className="settings-footer-row">
            <label style={{ color: '#888', fontSize: 12 }}>Avg over</label>
            <div style={{ display: 'flex', gap: 12, marginLeft: 'auto' }}>
              {([3, 5, 'all'] as LapWindow[]).map(w => (
                <label key={String(w)} style={radioLabel}>
                  <input
                    type="radio" name="flapwindow" value={String(w)}
                    checked={lapWindow === w}
                    onChange={() => onLapWindow(w)}
                  /> {w === 'all' ? 'All' : `${w} laps`}
                </label>
              ))}
            </div>
          </div>
        </div>

        <div className="settings-section">
          <div className="settings-section-title">Reserve</div>
          <div className="settings-footer-row">
            <label style={{ color: '#888', fontSize: 12 }}>Amount</label>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginLeft: 'auto' }}>
              <input
                type="number"
                min={0} max={99} step={0.5}
                value={reserveValue}
                onChange={e => onReserveValue(Math.max(0, parseFloat(e.target.value) || 0))}
                style={{
                  width: 52, background: '#111', border: '1px solid #333',
                  color: '#ccc', borderRadius: 4, padding: '2px 6px', fontSize: 12,
                }}
              />
              <div style={{ display: 'flex', gap: 8 }}>
                {(['L', 'laps'] as ReserveUnit[]).map(u => (
                  <label key={u} style={radioLabel}>
                    <input
                      type="radio" name="freserveunit" value={u}
                      checked={reserveUnit === u}
                      onChange={() => onReserveUnit(u)}
                    /> {u}
                  </label>
                ))}
              </div>
            </div>
          </div>
        </div>

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

        <div className="settings-footer-btns" style={{ display: 'flex', gap: 8 }}>
          <button className="settings-btn" onClick={onResetTracker}
            style={{ flex: 1, background: '#1a1a1a', border: '1px solid #333' }}>
            Reset tracker
          </button>
          <button className="settings-btn" onClick={onResetAll} style={{ flex: 1 }}>
            Reset all
          </button>
        </div>
      </div>
    </>
  )
}
