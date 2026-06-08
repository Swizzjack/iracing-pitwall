import {
  DndContext,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core'
import {
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
  arrayMove,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'

export type ColId = 'pos' | 'car' | 'num' | 'driver' | 'lap' | 'last' | 'best' | 'gap' | 'pit' | 'tire' | 'p2p' | 'sectors' | 'rating'

const COL_LABELS: Record<ColId, string> = {
  pos: 'Position', car: 'Car', num: '#', driver: 'Driver', lap: 'Lap',
  last: 'Last Lap', best: 'Best Lap', gap: 'Gap', pit: 'Pit Stop',
  tire: 'Tire', p2p: 'P2P', sectors: 'Sectors', rating: 'Rating',
}

interface RowProps {
  id: ColId
  visible: boolean
  width: number | undefined
  onToggle: () => void
  onResetWidth: () => void
}

function SortableRow({ id, visible, width, onToggle, onResetWidth }: RowProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id })
  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  }
  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`settings-col-row${isDragging ? ' dragging-row' : ''}`}
    >
      <span className="drag-handle" {...attributes} {...listeners}>⠿</span>
      <label className="toggle">
        <input type="checkbox" checked={visible} onChange={onToggle} />
        <span className="toggle-track" />
      </label>
      <span style={{ color: visible ? '#ccc' : '#555' }}>{COL_LABELS[id]}</span>
      {width != null ? (
        <button className="settings-btn" onClick={onResetWidth} title="Reset width">↺ {width}px</button>
      ) : (
        <span style={{ fontSize: 'var(--settings-fs-sm)', color: '#444' }}>auto</span>
      )}
    </div>
  )
}

interface Props {
  order: ColId[]
  hidden: Set<ColId>
  widths: Partial<Record<ColId, number>>
  fontScale: number
  onOrderChange: (order: ColId[]) => void
  onToggle: (id: ColId) => void
  onResetWidth: (id: ColId) => void
  onResetAllWidths: () => void
  onFontScaleChange: (scale: number) => void
  onResetAll: () => void
}

export function StandingsSettings({
  order, hidden, widths, fontScale,
  onOrderChange, onToggle, onResetWidth, onResetAllWidths,
  onFontScaleChange, onResetAll,
}: Props) {
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }))

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event
    if (over && active.id !== over.id) {
      const from = order.indexOf(active.id as ColId)
      const to = order.indexOf(over.id as ColId)
      onOrderChange(arrayMove(order, from, to))
    }
  }

  return (
    <>
      <div className="settings-col-list">
        <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
          <SortableContext items={order} strategy={verticalListSortingStrategy}>
            {order.map(id => (
              <SortableRow
                key={id}
                id={id}
                visible={!hidden.has(id)}
                width={widths[id]}
                onToggle={() => onToggle(id)}
                onResetWidth={() => onResetWidth(id)}
              />
            ))}
          </SortableContext>
        </DndContext>
      </div>

      <div className="settings-drawer-footer">
        <div className="settings-footer-row">
          <label>Font size</label>
          <input
            type="range"
            min={0.7}
            max={2.0}
            step={0.05}
            value={fontScale}
            onChange={e => onFontScaleChange(parseFloat(e.target.value))}
          />
          <span>{Math.round(fontScale * 100)}%</span>
        </div>
        <div className="settings-footer-btns">
          <button className="settings-btn" onClick={onResetAllWidths}>Reset widths</button>
          <button className="settings-btn" onClick={onResetAll}>Reset all</button>
        </div>
      </div>
    </>
  )
}
