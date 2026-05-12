import type { CornerData } from './Tire'
import { tempColor, wearColor, fmtPress, fmtTempShort } from './Tire'
import type { TireFieldId, TempUnit, PressUnit } from './TireSettings'

interface SharedProps {
  corners: CornerData[]
  fieldOrder: TireFieldId[]
  tempUnit: TempUnit
  pressUnit: PressUnit
  fontScale: number
}

function MiniCorner({ corner, tempUnit, pressUnit, fontScale }: {
  corner: CornerData
  tempUnit: TempUnit
  pressUnit: PressUnit
  fontScale: number
}) {
  const temps = corner.surfaceTemps ?? corner.carcassTemps
  const avg = (temps[0] + temps[1] + temps[2]) / 3
  const press = corner.hotPressure ?? corner.coldPressure
  const minWear = Math.min(...corner.wear)
  const fs = `calc(${11 * fontScale}px)`
  const smFs = `calc(${9 * fontScale}px)`

  return (
    <div className="tire-cluster-corner">
      <div className="tire-cluster-corner-label" style={{ fontSize: smFs }}>
        {corner.label}
      </div>

      {/* Temp strip */}
      <div style={{ display: 'flex', gap: 2, justifyContent: 'center', marginBottom: 4 }}>
        {temps.map((t, i) => (
          <div key={i} style={{
            width: 10, height: 10, borderRadius: 2,
            background: tempColor(t),
          }} />
        ))}
      </div>

      {/* Avg temp */}
      <div style={{
        textAlign: 'center', color: tempColor(avg),
        fontFamily: 'var(--font-mono)', fontSize: fs,
        marginBottom: 3,
      }}>
        {fmtTempShort(avg, tempUnit)}
      </div>

      {/* Pressure */}
      <div style={{
        textAlign: 'center', color: '#777',
        fontFamily: 'var(--font-mono)', fontSize: smFs,
        marginBottom: 4,
      }}>
        {fmtPress(press, pressUnit)}
      </div>

      {/* Wear bars */}
      <div style={{ display: 'flex', gap: 2, justifyContent: 'center', alignItems: 'flex-end', height: 16 }}>
        {corner.wear.map((w, i) => (
          <div key={i} style={{
            width: 7,
            height: Math.max(2, Math.round(w * 16)),
            background: wearColor(w),
            borderRadius: 1,
          }} />
        ))}
      </div>
      <div style={{
        textAlign: 'center', color: wearColor(minWear),
        fontFamily: 'var(--font-mono)', fontSize: smFs,
        marginTop: 2,
      }}>
        {Math.round(minWear * 100)}%
      </div>
    </div>
  )
}

export function TireCluster({ corners, tempUnit, pressUnit, fontScale }: SharedProps) {
  const [lf, rf, lr, rr] = corners

  return (
    <div className="tire-cluster-car">
      <div className="tire-cluster-axle">
        <MiniCorner corner={lf} tempUnit={tempUnit} pressUnit={pressUnit} fontScale={fontScale} />
        <div className="tire-cluster-body-front" />
        <MiniCorner corner={rf} tempUnit={tempUnit} pressUnit={pressUnit} fontScale={fontScale} />
      </div>
      <div className="tire-cluster-body-mid" />
      <div className="tire-cluster-axle">
        <MiniCorner corner={lr} tempUnit={tempUnit} pressUnit={pressUnit} fontScale={fontScale} />
        <div className="tire-cluster-body-rear" />
        <MiniCorner corner={rr} tempUnit={tempUnit} pressUnit={pressUnit} fontScale={fontScale} />
      </div>
    </div>
  )
}
