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

const FIELD_LABEL_SHORT: Record<TireFieldId, string> = {
  tempSurface: 'Surface',
  tempCarcass: 'Carcass',
  pressureCold: 'Cold P',
  pressureHot: 'Hot P',
  pressureDelta: 'ΔP',
  wear: 'Wear',
  wheelSpeed: 'Spd',
  rideHeight: 'RH',
}

function TempMini({ temps, unit, fontScale }: {
  temps: [number, number, number]
  unit: TempUnit
  fontScale: number
}) {
  const avg = (temps[0] + temps[1] + temps[2]) / 3
  return (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 3 }}>
      <div style={{ display: 'flex', gap: 2 }}>
        {temps.map((t, i) => (
          <div key={i} style={{ width: 7, height: 7, borderRadius: 2, background: tempColor(t) }} />
        ))}
      </div>
      <span style={{
        fontFamily: 'var(--font-mono)',
        fontSize: `calc(${12 * fontScale}px)`,
        color: tempColor(avg),
      }}>
        {fmtTempShort(avg, unit)}
      </span>
    </div>
  )
}

function WearMini({ wear, fontScale }: { wear: [number, number, number]; fontScale: number }) {
  const minW = Math.min(...wear)
  return (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 3 }}>
      <div style={{ display: 'flex', gap: 2, alignItems: 'flex-end', height: 14 }}>
        {wear.map((w, i) => (
          <div key={i} style={{
            width: 5,
            height: Math.max(2, Math.round(w * 14)),
            background: wearColor(w),
            borderRadius: 1,
          }} />
        ))}
      </div>
      <span style={{
        fontFamily: 'var(--font-mono)',
        fontSize: `calc(${12 * fontScale}px)`,
        color: wearColor(minW),
      }}>
        {Math.round(minW * 100)}%
      </span>
    </div>
  )
}

export function TireList({ corners, fieldOrder, tempUnit, pressUnit, fontScale }: SharedProps) {
  const fs = `calc(${12 * fontScale}px)`
  const labelFs = `calc(${11 * fontScale}px)`

  return (
    <div style={{ overflowX: 'auto' }}>
      <table className="tire-list-table">
        <thead>
          <tr>
            <th style={{ color: '#444', fontSize: labelFs, textAlign: 'left', padding: '3px 6px', fontWeight: 400 }} />
            {corners.map(c => (
              <th key={c.label} style={{
                color: '#777', fontSize: labelFs, textAlign: 'center',
                padding: '3px 6px', fontFamily: 'var(--font-mono)', fontWeight: 600,
              }}>
                {c.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {fieldOrder.map(field => {
            const hasAny = corners.some(c => {
              if (field === 'tempSurface') return c.surfaceTemps != null
              if (field === 'pressureHot' || field === 'pressureDelta') return c.hotPressure != null
              if (field === 'wheelSpeed') return c.wheelSpeed != null
              if (field === 'rideHeight') return c.rideHeight != null
              return true
            })
            if (!hasAny) return null

            return (
              <tr key={field} style={{ borderTop: '1px solid #1a1a1a' }}>
                <td style={{ color: '#555', fontSize: labelFs, padding: '5px 6px', whiteSpace: 'nowrap' }}>
                  {FIELD_LABEL_SHORT[field]}
                </td>
                {corners.map(c => {
                  const hotDelta = c.hotPressure != null ? c.hotPressure - c.coldPressure : null

                  let cell: React.ReactNode = null

                  if (field === 'tempSurface') {
                    if (c.surfaceTemps) {
                      cell = <TempMini temps={c.surfaceTemps} unit={tempUnit} fontScale={fontScale} />
                    } else {
                      cell = <span style={{ color: '#2a2a2a', fontSize: fs }}>—</span>
                    }
                  } else if (field === 'tempCarcass') {
                    cell = <TempMini temps={c.carcassTemps} unit={tempUnit} fontScale={fontScale} />
                  } else if (field === 'pressureCold') {
                    cell = (
                      <span style={{ color: '#999', fontFamily: 'var(--font-mono)', fontSize: fs }}>
                        {fmtPress(c.coldPressure, pressUnit)}
                      </span>
                    )
                  } else if (field === 'pressureHot') {
                    if (c.hotPressure != null) {
                      cell = (
                        <span style={{ color: '#ccc', fontFamily: 'var(--font-mono)', fontSize: fs }}>
                          {fmtPress(c.hotPressure, pressUnit)}
                        </span>
                      )
                    } else {
                      cell = <span style={{ color: '#2a2a2a', fontSize: fs }}>—</span>
                    }
                  } else if (field === 'pressureDelta') {
                    if (hotDelta != null) {
                      cell = (
                        <span style={{ color: pressDeltaColor(hotDelta), fontFamily: 'var(--font-mono)', fontSize: fs }}>
                          {hotDelta >= 0 ? '+' : ''}{fmtPress(hotDelta, pressUnit)}
                        </span>
                      )
                    } else {
                      cell = <span style={{ color: '#2a2a2a', fontSize: fs }}>—</span>
                    }
                  } else if (field === 'wear') {
                    cell = <WearMini wear={c.wear} fontScale={fontScale} />
                  } else if (field === 'wheelSpeed') {
                    if (c.wheelSpeed != null) {
                      cell = (
                        <span style={{ color: '#999', fontFamily: 'var(--font-mono)', fontSize: fs }}>
                          {(c.wheelSpeed * 3.6).toFixed(1)}
                        </span>
                      )
                    } else {
                      cell = <span style={{ color: '#2a2a2a', fontSize: fs }}>—</span>
                    }
                  } else if (field === 'rideHeight') {
                    if (c.rideHeight != null) {
                      cell = (
                        <span style={{ color: '#999', fontFamily: 'var(--font-mono)', fontSize: fs }}>
                          {(c.rideHeight * 1000).toFixed(1)}
                        </span>
                      )
                    } else {
                      cell = <span style={{ color: '#2a2a2a', fontSize: fs }}>—</span>
                    }
                  }

                  return (
                    <td key={c.label} style={{ padding: '5px 6px', textAlign: 'center', verticalAlign: 'middle' }}>
                      {cell}
                    </td>
                  )
                })}
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}
