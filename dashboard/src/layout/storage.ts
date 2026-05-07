import type { LayoutItem } from 'react-grid-layout'
import { REGISTRY } from './registry'

const KEY = 'iracing-dashboard-layout-v1'

export type StoredLayout = {
  visible: string[]
  layout: LayoutItem[]
}

// Positions for the 5 default widgets: Telemetry + Session + Inputs on row 0, Standings + TrackMap below
const DEFAULT_POSITIONS = [
  { x: 0, y: 0 },
  { x: 4, y: 0 },
  { x: 8, y: 0 },
  { x: 0, y: 9 },
  { x: 6, y: 9 },
]

export function buildDefaultLayout(): StoredLayout {
  const layout: LayoutItem[] = REGISTRY.map((def, i) => ({
    i: def.id,
    x: DEFAULT_POSITIONS[i]?.x ?? 0,
    y: DEFAULT_POSITIONS[i]?.y ?? 0,
    w: def.default.w,
    h: def.default.h,
    minW: def.default.minW,
    minH: def.default.minH,
  }))
  return { visible: REGISTRY.map((d) => d.id), layout }
}

export function loadLayout(): StoredLayout {
  try {
    const raw = localStorage.getItem(KEY)
    if (raw) {
      const parsed = JSON.parse(raw) as StoredLayout
      const validIds = new Set(REGISTRY.map((d) => d.id))
      const layout = (parsed.layout ?? []).filter((l) => validIds.has(l.i))
      const visible = (parsed.visible ?? []).filter((id) => validIds.has(id))
      if (visible.length > 0) return { visible, layout }
    }
  } catch { /* ignore */ }
  return buildDefaultLayout()
}

export function saveLayout(s: StoredLayout): void {
  try { localStorage.setItem(KEY, JSON.stringify(s)) } catch { /* ignore */ }
}

export function resetLayout(): StoredLayout {
  try { localStorage.removeItem(KEY) } catch { /* ignore */ }
  return buildDefaultLayout()
}
