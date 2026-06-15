export type SpeedUnit = 'kmh' | 'mph'

interface Props {
  unit: SpeedUnit
  fontScale: number
  onUnit: (u: SpeedUnit) => void
  onFontScale: (s: number) => void
  onResetAll: () => void
}

export function AccelerationSettings({ unit, fontScale, onUnit, onFontScale, onResetAll }: Props) {
  const radioLabel: React.CSSProperties = { display: 'flex', alignItems: 'center', gap: 5, cursor: 'pointer', color: '#ccc', fontSize: 'var(--settings-fs)' }

  return (
    <div className="settings-drawer-footer">
      <div className="settings-section">
        <div className="settings-section-title">Units</div>
        <div className="settings-footer-row">
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Speed</label>
          <div style={{ display: 'flex', gap: 12, marginLeft: 'auto' }}>
            <label style={radioLabel}>
              <input type="radio" name="accel-unit" value="kmh" checked={unit === 'kmh'} onChange={() => onUnit('kmh')} /> Metric (km/h)
            </label>
            <label style={radioLabel}>
              <input type="radio" name="accel-unit" value="mph" checked={unit === 'mph'} onChange={() => onUnit('mph')} /> Imperial (mph)
            </label>
          </div>
        </div>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">Display</div>
        <div className="settings-footer-row">
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Font size</label>
          <input
            type="range" min={0.7} max={2.0} step={0.05}
            value={fontScale}
            onChange={e => onFontScale(parseFloat(e.target.value))}
          />
          <span style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>
            {Math.round(fontScale * 100)}%
          </span>
        </div>
      </div>

      <div className="settings-footer-btns">
        <button className="settings-btn" onClick={onResetAll}>Reset all</button>
      </div>
    </div>
  )
}
