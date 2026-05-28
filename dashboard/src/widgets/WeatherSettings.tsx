import {
  DndContext, closestCenter, PointerSensor, useSensor, useSensors, type DragEndEvent,
} from '@dnd-kit/core'
import {
  SortableContext, useSortable, verticalListSortingStrategy, arrayMove,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'

export type WeatherFieldId =
  | 'skies' | 'airTemp' | 'trackTemp' | 'wetness' | 'declaredWet'
  | 'precipitation' | 'wind' | 'humidity' | 'fog' | 'pressure' | 'density'
  | 'timeOfDay' | 'weatherType' | 'rubberState'

export type TempUnit = 'C' | 'F'
export type SpeedUnit = 'kmh' | 'mph'

const FIELD_LABELS: Record<WeatherFieldId, string> = {
  skies: 'Sky Conditions',
  airTemp: 'Air Temp',
  trackTemp: 'Track Temp',
  wetness: 'Track Wetness',
  declaredWet: 'Wet Declared',
  precipitation: 'Precipitation',
  wind: 'Wind',
  humidity: 'Humidity',
  fog: 'Fog Level',
  pressure: 'Air Pressure',
  density: 'Air Density',
  timeOfDay: 'Time of Day',
  weatherType: 'Weather Mode',
  rubberState: 'Track Rubber',
}

export const DEFAULT_FIELD_ORDER: WeatherFieldId[] = [
  'skies', 'airTemp', 'trackTemp', 'wetness', 'rubberState', 'declaredWet',
  'precipitation', 'wind', 'humidity', 'fog', 'pressure', 'density',
  'timeOfDay', 'weatherType',
]

interface RowProps {
  id: WeatherFieldId
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
  order: WeatherFieldId[]
  hidden: Set<WeatherFieldId>
  tempUnit: TempUnit
  speedUnit: SpeedUnit
  fontScale: number
  onOrderChange: (o: WeatherFieldId[]) => void
  onToggle: (id: WeatherFieldId) => void
  onTempUnit: (u: TempUnit) => void
  onSpeedUnit: (u: SpeedUnit) => void
  onFontScale: (scale: number) => void
  onResetAll: () => void
}

export function WeatherSettings({
  order, hidden, tempUnit, speedUnit, fontScale,
  onOrderChange, onToggle, onTempUnit, onSpeedUnit, onFontScale, onResetAll,
}: Props) {
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }))

  function handleDragEnd(e: DragEndEvent) {
    const { active, over } = e
    if (over && active.id !== over.id) {
      const from = order.indexOf(active.id as WeatherFieldId)
      const to = order.indexOf(over.id as WeatherFieldId)
      onOrderChange(arrayMove(order, from, to))
    }
  }

  const radioLabel: React.CSSProperties = { display: 'flex', alignItems: 'center', gap: 5, cursor: 'pointer', color: '#ccc', fontSize: 'var(--settings-fs)' }

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
          <div className="settings-section-title">Display</div>
          <div className="settings-footer-row">
            <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Font size</label>
            <input
              type="range"
              min={0.7}
              max={2.0}
              step={0.05}
              value={fontScale}
              onChange={e => onFontScale(parseFloat(e.target.value))}
            />
            <span style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>{Math.round(fontScale * 100)}%</span>
          </div>
        </div>
        <div className="settings-section">
          <div className="settings-section-title">Units</div>
          <div className="settings-footer-row">
            <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Temperature</label>
            <div style={{ display: 'flex', gap: 12, marginLeft: 'auto' }}>
              <label style={radioLabel}>
                <input type="radio" name="wtemp" value="C" checked={tempUnit === 'C'} onChange={() => onTempUnit('C')} /> °C
              </label>
              <label style={radioLabel}>
                <input type="radio" name="wtemp" value="F" checked={tempUnit === 'F'} onChange={() => onTempUnit('F')} /> °F
              </label>
            </div>
          </div>
          <div className="settings-footer-row">
            <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Wind speed</label>
            <div style={{ display: 'flex', gap: 12, marginLeft: 'auto' }}>
              <label style={radioLabel}>
                <input type="radio" name="wspeed" value="kmh" checked={speedUnit === 'kmh'} onChange={() => onSpeedUnit('kmh')} /> km/h
              </label>
              <label style={radioLabel}>
                <input type="radio" name="wspeed" value="mph" checked={speedUnit === 'mph'} onChange={() => onSpeedUnit('mph')} /> mph
              </label>
            </div>
          </div>
        </div>
        <div className="settings-footer-btns">
          <button className="settings-btn" onClick={onResetAll}>Reset all</button>
        </div>
      </div>
    </>
  )
}
