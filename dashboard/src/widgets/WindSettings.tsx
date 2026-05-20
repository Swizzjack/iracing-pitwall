export type SpeedUnit = 'kmh' | 'mph' | 'ms'

export const SPEED_UNIT_DEFAULT: SpeedUnit = 'kmh'

interface Props {
  speedUnit: SpeedUnit
  showCompass: boolean
  showStreamlines: boolean
  fontScale: number
  onSpeedUnit: (u: SpeedUnit) => void
  onShowCompass: (v: boolean) => void
  onShowStreamlines: (v: boolean) => void
  onFontScale: (v: number) => void
  onResetAll: () => void
}

export function WindSettings({
  speedUnit, showCompass, showStreamlines, fontScale,
  onSpeedUnit, onShowCompass, onShowStreamlines, onFontScale, onResetAll,
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
        <div className="settings-section-title">Display</div>
        <label style={{ display: 'flex', alignItems: 'center', gap: 10, fontSize: 12, color: '#aaa', cursor: 'pointer' }}>
          <span className="toggle">
            <input type="checkbox" checked={showCompass} onChange={e => onShowCompass(e.target.checked)} />
            <span className="toggle-track" />
          </span>
          Absolute compass ring
        </label>
        <label style={{ display: 'flex', alignItems: 'center', gap: 10, fontSize: 12, color: '#aaa', cursor: 'pointer', marginTop: 8 }}>
          <span className="toggle">
            <input type="checkbox" checked={showStreamlines} onChange={e => onShowStreamlines(e.target.checked)} />
            <span className="toggle-track" />
          </span>
          Streamlines
        </label>
      </div>

      <div className="settings-drawer-footer">
        <div className="settings-footer-row">
          <label>Font Scale</label>
          <input
            type="range" min="0.8" max="1.4" step="0.05"
            value={fontScale}
            onChange={e => onFontScale(parseFloat(e.target.value))}
          />
          <span>{fontScale.toFixed(2)}×</span>
        </div>
        <div className="settings-footer-btns">
          <button className="settings-btn" onClick={onResetAll}>Reset All</button>
        </div>
      </div>
    </>
  )
}
