interface Props {
  fontScale: number
  onFontScale: (s: number) => void
  onResetAll: () => void
}

export function EngineerTranscriptSettings({ fontScale, onFontScale, onResetAll }: Props) {
  return (
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
