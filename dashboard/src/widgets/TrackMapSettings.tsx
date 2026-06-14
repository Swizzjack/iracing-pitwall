import { useState } from 'react'

interface Props {
  trackWidth: number
  carRadius: number
  sfLength: number
  sectorShow: boolean
  sectorLength: number
  sectorLiveColors: boolean
  sectorFontSize: number
  fontSize: number
  tiltDeg: number
  zExag: number
  hasMap: boolean
  trackKey?: string
  onTrackWidth: (v: number) => void
  onCarRadius: (v: number) => void
  onSfLength: (v: number) => void
  onSectorShow: (v: boolean) => void
  onSectorLength: (v: number) => void
  onSectorLiveColors: (v: boolean) => void
  onSectorFontSize: (v: number) => void
  onFontSize: (v: number) => void
  onTiltDeg: (v: number) => void
  onZExag: (v: number) => void
  onResetAll: () => void
  onDelete: (trackKey: string) => void
}

export function TrackMapSettings({
  trackWidth, carRadius, sfLength, sectorShow, sectorLength, sectorLiveColors, sectorFontSize, fontSize, tiltDeg, zExag,
  hasMap, trackKey,
  onTrackWidth, onCarRadius, onSfLength, onSectorShow, onSectorLength, onSectorLiveColors, onSectorFontSize, onFontSize,
  onTiltDeg, onZExag, onResetAll, onDelete,
}: Props) {
  const [confirmDelete, setConfirmDelete] = useState(false)

  // The armed state is only meaningful while a map exists — derived instead
  // of being reset via an effect when the map disappears.
  const showConfirm = confirmDelete && hasMap

  return (
    <>
      <div className="settings-section">
        <div className="settings-section-title">Track</div>
        <div className="settings-footer-row">
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Width</label>
          <input type="range" min={2} max={24} step={1} value={trackWidth}
            onChange={e => onTrackWidth(+e.target.value)} />
          <span style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>{trackWidth}px</span>
        </div>
        <div className="settings-footer-row">
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>S/F line</label>
          <input type="range" min={4} max={40} step={1} value={sfLength}
            onChange={e => onSfLength(+e.target.value)} />
          <span style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>{sfLength}px</span>
        </div>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">Cars</div>
        <div className="settings-footer-row">
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Dot size</label>
          <input type="range" min={3} max={16} step={1} value={carRadius}
            onChange={e => onCarRadius(+e.target.value)} />
          <span style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>{carRadius}px</span>
        </div>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">Sectors</div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <label className="toggle">
            <input type="checkbox" checked={sectorShow}
              onChange={e => onSectorShow(e.target.checked)} />
            <span className="toggle-track" />
          </label>
          <span style={{ color: sectorShow ? '#ccc' : '#555', fontSize: 'var(--settings-fs)' }}>Show sectors</span>
        </div>
        <div className="settings-footer-row">
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Line length</label>
          <input type="range" min={4} max={40} step={1} value={sectorLength}
            onChange={e => onSectorLength(+e.target.value)} />
          <span style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>{sectorLength}px</span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <label className="toggle">
            <input type="checkbox" checked={sectorLiveColors}
              onChange={e => onSectorLiveColors(e.target.checked)} />
            <span className="toggle-track" />
          </label>
          <span style={{ color: sectorLiveColors ? '#ccc' : '#555', fontSize: 'var(--settings-fs)' }}>Live performance colors</span>
        </div>
        <div className="settings-footer-row">
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Label size</label>
          <input type="range" min={6} max={18} step={1} value={sectorFontSize}
            onChange={e => onSectorFontSize(+e.target.value)} />
          <span style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>{sectorFontSize}px</span>
        </div>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">Info overlay</div>
        <div className="settings-footer-row">
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Font size</label>
          <input type="range" min={8} max={20} step={1} value={fontSize}
            onChange={e => onFontSize(+e.target.value)} />
          <span style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>{fontSize}px</span>
        </div>
      </div>

      <div className="settings-section">
        <div className="settings-section-title">3D view</div>
        <div className="settings-footer-row">
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Tilt</label>
          <input type="range" min={0} max={75} step={1} value={tiltDeg}
            onChange={e => onTiltDeg(+e.target.value)} />
          <span style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>{tiltDeg}°</span>
        </div>
        <div className="settings-footer-row">
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Elevation</label>
          <input type="range" min={1} max={50} step={1} value={zExag}
            onChange={e => onZExag(+e.target.value)} />
          <span style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>{zExag}×</span>
        </div>
      </div>

      <div className="settings-drawer-footer">
        <div className="settings-footer-btns">
          <button className="settings-btn" onClick={onResetAll}>Reset all</button>
          {hasMap && (
            showConfirm ? (
              <button
                className="settings-btn settings-btn-active"
                onClick={() => { if (trackKey) onDelete(trackKey); setConfirmDelete(false) }}
              >
                Really delete?
              </button>
            ) : (
              <button
                className="header-btn header-btn-danger"
                onClick={() => setConfirmDelete(true)}
                title="Delete the cached track map — a new lap will be recorded"
              >
                🗑 Delete map
              </button>
            )
          )}
        </div>
      </div>
    </>
  )
}
