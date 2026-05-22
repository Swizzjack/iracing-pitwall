export type SpeedUnit = 'kmh' | 'mph' | 'ms'
export type MarkerStyle = 'arrow' | 'chevron'

export const SPEED_UNIT_DEFAULT: SpeedUnit = 'kmh'
export const MARKER_STYLE_DEFAULT: MarkerStyle = 'arrow'

interface Props {
  speedUnit: SpeedUnit
  markerStyle: MarkerStyle
  fontScale: number
  onSpeedUnit: (u: SpeedUnit) => void
  onMarkerStyle: (s: MarkerStyle) => void
  onFontScale: (v: number) => void
  onResetAll: () => void
}

export function WindSettings({
  speedUnit, markerStyle, fontScale,
  onSpeedUnit, onMarkerStyle, onFontScale, onResetAll,
}: Props) {
  return (
    <>
      <div className="settings-section">
        <div className="settings-section-title">Speed Unit</div>
        <div style={{ display: 'flex', gap: 6 }}>
          {(['kmh', 'mph', 'ms'] as SpeedUnit[]).map(u => (
            <button
              key={u}
              className={`settings-btn${speedUnit === u ? ' settings-btn-active' : ''}`}
              onClick={() => onSpeedUnit(u)}
            >
              {u === 'kmh' ? 'km/h' : u === 'mph' ? 'mph' : 'm/s'}
            </button>
          ))}
        </div>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">Wind Marker</div>
        <div style={{ display: 'flex', gap: 6 }}>
          <button
            className={`settings-btn${markerStyle === 'arrow' ? ' settings-btn-active' : ''}`}
            onClick={() => onMarkerStyle('arrow')}
          >Arrow</button>
          <button
            className={`settings-btn${markerStyle === 'chevron' ? ' settings-btn-active' : ''}`}
            onClick={() => onMarkerStyle('chevron')}
          >Chevron</button>
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

      <div className="settings-drawer-footer">
        <div className="settings-footer-btns">
          <button className="settings-btn" onClick={onResetAll}>Reset All</button>
        </div>
      </div>
    </>
  )
}
