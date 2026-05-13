export type SortOrder = 'newest' | 'oldest' | 'fastest'

const radioLabel: React.CSSProperties = {
  display: 'flex', alignItems: 'center', gap: 4,
  color: '#aaa', fontSize: 12, cursor: 'pointer',
}

interface Props {
  sortOrder: SortOrder
  fontScale: number
  onSortOrder: (v: SortOrder) => void
  onFontScale: (v: number) => void
}

export function LapHistorySettings({ sortOrder, fontScale, onSortOrder, onFontScale }: Props) {
  return (
    <>
      <div className="settings-drawer-footer">
        <div className="settings-section">
          <div className="settings-section-title">Sortierung</div>
          <div className="settings-footer-row">
            <label style={{ color: '#888', fontSize: 12 }}>Reihenfolge</label>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6, marginLeft: 'auto', alignItems: 'flex-end' }}>
              {(['newest', 'oldest', 'fastest'] as SortOrder[]).map(v => (
                <label key={v} style={radioLabel}>
                  <input
                    type="radio"
                    name="lhsort"
                    value={v}
                    checked={sortOrder === v}
                    onChange={() => onSortOrder(v)}
                  />
                  {v === 'newest' ? 'Neueste' : v === 'oldest' ? 'Älteste' : 'Schnellste'}
                </label>
              ))}
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
      </div>
    </>
  )
}
