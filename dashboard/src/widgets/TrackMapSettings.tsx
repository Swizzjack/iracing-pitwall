interface Props {
  trackWidth: number
  carRadius: number
  sfLength: number
  sectorShow: boolean
  sectorLength: number
  fontSize: number
  onTrackWidth: (v: number) => void
  onCarRadius: (v: number) => void
  onSfLength: (v: number) => void
  onSectorShow: (v: boolean) => void
  onSectorLength: (v: number) => void
  onFontSize: (v: number) => void
  onResetAll: () => void
}

export function TrackMapSettings({
  trackWidth, carRadius, sfLength, sectorShow, sectorLength, fontSize,
  onTrackWidth, onCarRadius, onSfLength, onSectorShow, onSectorLength, onFontSize,
  onResetAll,
}: Props) {
  return (
    <>
      <div className="settings-section">
        <div className="settings-section-title">Track</div>
        <div className="settings-footer-row">
          <label>Width</label>
          <input type="range" min={2} max={24} step={1} value={trackWidth}
            onChange={e => onTrackWidth(+e.target.value)} />
          <span>{trackWidth}px</span>
        </div>
        <div className="settings-footer-row">
          <label>S/F line</label>
          <input type="range" min={4} max={40} step={1} value={sfLength}
            onChange={e => onSfLength(+e.target.value)} />
          <span>{sfLength}px</span>
        </div>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">Cars</div>
        <div className="settings-footer-row">
          <label>Dot size</label>
          <input type="range" min={3} max={16} step={1} value={carRadius}
            onChange={e => onCarRadius(+e.target.value)} />
          <span>{carRadius}px</span>
        </div>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">Sectors</div>
        <div className="settings-footer-row">
          <label>Show</label>
          <label className="toggle">
            <input type="checkbox" checked={sectorShow}
              onChange={e => onSectorShow(e.target.checked)} />
            <span className="toggle-track" />
          </label>
        </div>
        <div className="settings-footer-row">
          <label>Line length</label>
          <input type="range" min={4} max={40} step={1} value={sectorLength}
            onChange={e => onSectorLength(+e.target.value)} />
          <span>{sectorLength}px</span>
        </div>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">Info overlay</div>
        <div className="settings-footer-row">
          <label>Font size</label>
          <input type="range" min={8} max={20} step={1} value={fontSize}
            onChange={e => onFontSize(+e.target.value)} />
          <span>{fontSize}px</span>
        </div>
      </div>

      <div className="settings-drawer-footer">
        <div className="settings-footer-btns">
          <button className="settings-btn" onClick={onResetAll}>Reset all</button>
        </div>
      </div>
    </>
  )
}
