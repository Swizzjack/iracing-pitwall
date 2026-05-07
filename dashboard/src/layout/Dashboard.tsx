import GridLayout, { useContainerWidth } from 'react-grid-layout'
import type { LayoutItem } from 'react-grid-layout'
import { REGISTRY, type WidgetData } from './registry'

interface Props {
  data: WidgetData
  visible: string[]
  layout: LayoutItem[]
  editing: boolean
  onLayoutChange: (layout: readonly LayoutItem[]) => void
  onRemove: (id: string) => void
}

function getItemLayout(id: string, layout: LayoutItem[]): LayoutItem {
  const saved = layout.find((l) => l.i === id)
  const def = REGISTRY.find((d) => d.id === id)!
  return saved
    ? { ...saved, minW: def.default.minW, minH: def.default.minH }
    : { i: id, x: 0, y: 9999, w: def.default.w, h: def.default.h, minW: def.default.minW, minH: def.default.minH }
}

export function Dashboard({ data, visible, layout, editing, onLayoutChange, onRemove }: Props) {
  const { width, containerRef } = useContainerWidth()

  return (
    <div ref={containerRef as React.RefObject<HTMLDivElement>}>
      <GridLayout
        className={editing ? 'editing' : ''}
        width={width}
        layout={visible.map((id) => getItemLayout(id, layout))}
        gridConfig={{ cols: 12, rowHeight: 36, margin: [12, 12], containerPadding: [12, 12], maxRows: Infinity }}
        dragConfig={{ enabled: editing, handle: '.card > h2' }}
        resizeConfig={{ enabled: editing, handles: ['se' as const] }}
        onLayoutChange={onLayoutChange}
      >
        {visible.map((id) => {
          const def = REGISTRY.find((d) => d.id === id)
          if (!def) return null
          return (
            <div key={id}>
              {def.render(data)}
              {editing && (
                <button
                  className="widget-remove"
                  onClick={() => onRemove(id)}
                  title="Widget entfernen"
                >
                  ×
                </button>
              )}
            </div>
          )
        })}
      </GridLayout>
    </div>
  )
}
