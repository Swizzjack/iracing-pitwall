import type { CornerData } from './Tire'
import { tempColor, wearColor, pressDeltaColor, fmtPress, fmtTempShort } from './Tire'
import type { TireFieldId, TempUnit, PressUnit } from './TireSettings'

interface SharedProps {
  corners: CornerData[]
  fieldOrder: TireFieldId[]
  tempUnit: TempUnit
  pressUnit: PressUnit
  fontScale: number
}

function TempStrip({ temps, unit, fontScale }: {
  temps: [number, number, number]
  unit: TempUnit
  fontScale: number
}) {
  const avg = (temps[0] + temps[1] + temps[2]) / 3
  return (
    <div className="tire-temp-strip">
      <div className="tire-temp-cells">
        {temps.map((t, i) => (
          <div key={i} className="tire-temp-cell">
            <div className="tire-temp-block" style={{ background: tempColor(t) }} />
            <span className="tire-temp-val" style={{ color: tempColor(t), fontSize: `calc(${9 * fontScale}px)` }}>
              {fmtTempShort(t, unit)}
            </span>
          </div>
        ))}
      </div>
      <span className="tire-temp-avg" style={{ color: tempColor(avg), fontSize: `calc(${13 * fontScale}px)` }}>
        {fmtTempShort(avg, unit)}
      </span>
    </div>
  )
}

function WearBars({ wear, fontScale }: { wear: [number, number, number]; fontScale: number }) {
  return (
    <div className="tire-wear-bars">
      {wear.map((w, i) => (
        <div key={i} className="tire-wear-bar-col">
          <div className="tire-wear-bar-track">
            <div
              className="tire-wear-bar-fill"
              style={{ height: `${Math.round(w * 100)}%`, background: wearColor(w) }}
            />
          </div>
          <span className="tire-wear-label" style={{ color: wearColor(w), fontSize: `calc(${9 * fontScale}px)` }}>
            {Math.round(w * 100)}
          </span>
        </div>
      ))}
    </div>
  )
}

function CornerCard({ corner, fieldOrder, tempUnit, pressUnit, fontScale }: {
  corner: CornerData
  fieldOrder: TireFieldId[]
  tempUnit: TempUnit
  pressUnit: PressUnit
  fontScale: number
}) {
  const fs = `calc(${12 * fontScale}px)`
  const labelFs = `calc(${10 * fontScale}px)`
  const hotDelta = corner.hotPressure != null ? corner.hotPressure - corner.coldPressure : null

  return (
    <div className="tire-bento-card">
      <div className="tire-bento-corner-label" style={{ fontSize: `calc(${11 * fontScale}px)` }}>
        {corner.label}
      </div>

      {fieldOrder.map(field => {
        switch (field) {
          case 'tempSurface':
            if (!corner.surfaceTemps) return null
            return (
              <div key={field} className="tire-bento-section">
                <span className="tire-field-label" style={{ fontSize: labelFs }}>Surface</span>
                <TempStrip temps={corner.surfaceTemps} unit={tempUnit} fontScale={fontScale} />
              </div>
            )
          case 'tempCarcass':
            return (
              <div key={field} className="tire-bento-section">
                <span className="tire-field-label" style={{ fontSize: labelFs }}>Carcass</span>
                <TempStrip temps={corner.carcassTemps} unit={tempUnit} fontScale={fontScale} />
              </div>
            )
          case 'pressureCold':
            return (
              <div key={field} className="tire-bento-section tire-kv-row">
                <span className="tire-field-label" style={{ fontSize: labelFs }}>Cold</span>
                <span style={{ color: '#999', fontFamily: 'var(--font-mono)', fontSize: fs }}>
                  {fmtPress(corner.coldPressure, pressUnit)}
                </span>
              </div>
            )
          case 'pressureHot':
            if (corner.hotPressure == null) return null
            return (
              <div key={field} className="tire-bento-section tire-kv-row">
                <span className="tire-field-label" style={{ fontSize: labelFs }}>Hot</span>
                <span style={{ color: '#ccc', fontFamily: 'var(--font-mono)', fontSize: fs }}>
                  {fmtPress(corner.hotPressure, pressUnit)}
                </span>
              </div>
            )
          case 'pressureDelta':
            if (hotDelta == null) return null
            return (
              <div key={field} className="tire-bento-section tire-kv-row">
                <span className="tire-field-label" style={{ fontSize: labelFs }}>ΔP</span>
                <span style={{ color: pressDeltaColor(hotDelta), fontFamily: 'var(--font-mono)', fontSize: fs }}>
                  {hotDelta >= 0 ? '+' : ''}{fmtPress(hotDelta, pressUnit)}
                </span>
              </div>
            )
          case 'wear':
            return (
              <div key={field} className="tire-bento-section">
                <span className="tire-field-label" style={{ fontSize: labelFs }}>Wear</span>
                <WearBars wear={corner.wear} fontScale={fontScale} />
              </div>
            )
          case 'wheelSpeed':
            if (corner.wheelSpeed == null) return null
            return (
              <div key={field} className="tire-bento-section tire-kv-row">
                <span className="tire-field-label" style={{ fontSize: labelFs }}>Spd</span>
                <span style={{ color: '#999', fontFamily: 'var(--font-mono)', fontSize: fs }}>
                  {(corner.wheelSpeed * 3.6).toFixed(1)} km/h
                </span>
              </div>
            )
          case 'rideHeight':
            if (corner.rideHeight == null) return null
            return (
              <div key={field} className="tire-bento-section tire-kv-row">
                <span className="tire-field-label" style={{ fontSize: labelFs }}>RH</span>
                <span style={{ color: '#999', fontFamily: 'var(--font-mono)', fontSize: fs }}>
                  {(corner.rideHeight * 1000).toFixed(1)} mm
                </span>
              </div>
            )
          default:
            return null
        }
      })}
    </div>
  )
}

export function TireBento({ corners, fieldOrder, tempUnit, pressUnit, fontScale }: SharedProps) {
  return (
    <div className="tire-bento-grid">
      {corners.map(c => (
        <CornerCard
          key={c.label}
          corner={c}
          fieldOrder={fieldOrder}
          tempUnit={tempUnit}
          pressUnit={pressUnit}
          fontScale={fontScale}
        />
      ))}
    </div>
  )
}
