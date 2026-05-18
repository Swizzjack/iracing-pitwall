import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import type { ResolvedField } from './Electronics'

interface Props {
  snap: TelemetrySnapshot
  fields: ResolvedField[]
  fontScale: number
}

// ─── Hero Block ───────────────────────────────────────────────────────────────

function gearLabel(g: number) {
  if (g === -1) return 'R'
  if (g === 0) return 'N'
  return String(g)
}

function rpmColor(rpm: number, shift?: number | null, blink?: number | null) {
  if (blink != null && rpm >= blink) return '#ef4444'
  if (shift != null && rpm >= shift) return '#facc15'
  return '#22c55e'
}

function RpmBar({ snap, fontScale }: { snap: TelemetrySnapshot; fontScale: number }) {
  const maxRpm = snap.shiftLightLastRpm ?? Math.max(snap.rpm * 1.05, 8000)
  const pct = Math.min(snap.rpm / maxRpm, 1)
  const shiftPct = snap.shiftLightShiftRpm != null ? snap.shiftLightShiftRpm / maxRpm : 0.82
  const blinkPct = snap.shiftLightBlinkRpm != null ? snap.shiftLightBlinkRpm / maxRpm : 0.92
  const color = rpmColor(snap.rpm, snap.shiftLightShiftRpm, snap.shiftLightBlinkRpm)
  const isBlinking = snap.shiftLightBlinkRpm != null && snap.rpm >= snap.shiftLightBlinkRpm

  return (
    <div style={{ flex: 1, minWidth: 0 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
        <span style={{ color: '#555', fontSize: `calc(${10 * fontScale}px)`, textTransform: 'uppercase', letterSpacing: '0.06em' }}>RPM</span>
        <span style={{
          color,
          fontFamily: 'var(--font-mono)',
          fontSize: `calc(${13 * fontScale}px)`,
          animation: isBlinking ? 'elec-blink 0.15s ease-in-out infinite alternate' : undefined,
        }}>
          {snap.rpm.toFixed(0)}
        </span>
      </div>
      {/* RPM bar with segment markers */}
      <div className="elec-rpm-bar" style={{ position: 'relative', height: `calc(${8 * fontScale}px)`, background: '#111', borderRadius: 3, overflow: 'hidden' }}>
        <div style={{
          position: 'absolute', inset: 0, width: `${pct * 100}%`,
          background: color,
          borderRadius: 3,
          transition: isBlinking ? undefined : 'width 0.05s linear',
          animation: isBlinking ? 'elec-blink-bar 0.15s ease-in-out infinite alternate' : undefined,
        }} />
        {/* Shift threshold markers */}
        {[shiftPct, blinkPct].map((mp, i) => (
          <div key={i} style={{
            position: 'absolute', top: 0, bottom: 0,
            left: `${mp * 100}%`, width: 1,
            background: i === 0 ? '#facc1560' : '#ef444460',
          }} />
        ))}
      </div>
    </div>
  )
}

interface HeroProps extends Omit<Props, 'fields'> {
  showGear: boolean
  showRpm: boolean
  showP2p: boolean
}

function Hero({ snap, fontScale, showGear, showRpm, showP2p }: HeroProps) {
  const hasDrs = snap.drsStatus != null
  const drsActive = (snap.drsStatus ?? 0) >= 2
  const drsAvail = (snap.drsStatus ?? 0) >= 1
  const hasP2pData = snap.p2pCount != null || snap.p2pStatus != null
  const p2pActive = snap.p2pStatus === true

  return (
    <div className="elec-hero" style={{ marginBottom: 10 }}>
      {(showGear || showRpm) && (
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          {showGear && (
            <div style={{
              fontFamily: 'var(--font-mono)',
              fontSize: `calc(${36 * fontScale}px)`,
              fontWeight: 700, lineHeight: 1,
              color: snap.gear === -1 ? '#ef4444' : snap.gear === 0 ? '#facc15' : '#f0f0f0',
              minWidth: `calc(${30 * fontScale}px)`, textAlign: 'center',
              textShadow: '0 0 20px rgba(249,115,22,0.3)',
            }}>
              {gearLabel(snap.gear)}
            </div>
          )}
          {showRpm && <RpmBar snap={snap} fontScale={fontScale} />}
        </div>
      )}

      {/* Status LEDs row */}
      {(hasDrs || (showP2p && hasP2pData) || snap.onPitRoad) && (
        <div style={{ display: 'flex', gap: 6, marginTop: showGear || showRpm ? 7 : 0, flexWrap: 'wrap' }}>
          {hasDrs && (
            <span className={`elec-led${drsActive ? ' elec-led-active-green' : drsAvail ? ' elec-led-ready' : ''}`}>
              DRS
            </span>
          )}
          {snap.onPitRoad && (snap.engineWarnings & 0x10) !== 0 && (
            <span className="elec-led elec-led-active-orange">PIT LIM</span>
          )}
          {showP2p && hasP2pData && (
            <span className={`elec-led${p2pActive ? ' elec-led-active-blue' : ''}`}>
              P2P{snap.p2pCount != null ? ` ×${snap.p2pCount}` : ''}
            </span>
          )}
        </div>
      )}
    </div>
  )
}

// ─── Bento Cards ──────────────────────────────────────────────────────────────

function PillStepper({ value, label, color = '#22c55e' }: { value: number; label: string; color?: string }) {
  const level = value > 1.5 ? Math.round(value) : Math.round(value * 12)
  const maxDots = 12
  const dots = Math.min(level, maxDots)
  return (
    <div style={{ marginBottom: 4 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 2 }}>
        <span style={{ color: '#666', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)' }}>{label}</span>
        <span style={{ color, fontFamily: 'var(--font-mono)', fontSize: 'calc(var(--widget-font-scale, 1) * 12px)' }}>{level}</span>
      </div>
      <div style={{ display: 'flex', gap: 2 }}>
        {Array.from({ length: maxDots }, (_, i) => (
          <div key={i} style={{
            width: 6, height: 6, borderRadius: 1,
            background: i < dots ? color : '#1e1e1e',
            transition: 'background 0.1s',
          }} />
        ))}
      </div>
    </div>
  )
}

function TempBar({ label, value, low, high, unit = '°C' }: {
  label: string; value: number; low: number; high: number; unit?: string
}) {
  const pct = Math.max(0, Math.min((value - (low * 0.7)) / (high * 1.1 - low * 0.7), 1))
  const color = value > high ? '#ef4444' : value > high * 0.9 ? '#facc15' : '#22c55e'
  return (
    <div style={{ marginBottom: 5 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 2 }}>
        <span style={{ color: '#666', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)' }}>{label}</span>
        <span style={{ color, fontFamily: 'var(--font-mono)', fontSize: 'calc(var(--widget-font-scale, 1) * 12px)' }}>
          {value.toFixed(1)}<span style={{ color: '#555', fontSize: 'calc(var(--widget-font-scale, 1) * 10px)' }}>{unit}</span>
        </span>
      </div>
      <div style={{ height: 3, background: '#1a1a1a', borderRadius: 2 }}>
        <div style={{ height: '100%', width: `${pct * 100}%`, background: color, borderRadius: 2, transition: 'width 0.3s' }} />
      </div>
    </div>
  )
}

function EngineWarningsLeds({ warnings }: { warnings: number }) {
  const bits = [
    { bit: 0x01, label: 'H₂O', warn: true },
    { bit: 0x02, label: 'FUEL-P', warn: true },
    { bit: 0x04, label: 'OIL-P', warn: true },
    { bit: 0x08, label: 'STALL', warn: true },
    { bit: 0x10, label: 'PIT', warn: false },
    { bit: 0x20, label: 'REV', warn: false },
    { bit: 0x40, label: 'OIL-T', warn: true },
  ]
  return (
    <div>
      <span style={{ color: '#444', fontSize: 'calc(var(--widget-font-scale, 1) * 10px)', textTransform: 'uppercase', letterSpacing: '0.06em', display: 'block', marginBottom: 5 }}>
        Warnings
      </span>
      <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
        {bits.map(({ bit, label, warn }) => {
          const active = (warnings & bit) !== 0
          return (
            <span key={bit} style={{
              fontSize: 'calc(var(--widget-font-scale, 1) * 10px)', fontFamily: 'var(--font-mono)',
              padding: '2px 4px', borderRadius: 3,
              background: active ? (warn ? '#ef444418' : '#f9731618') : '#0f0f0f',
              color: active ? (warn ? '#ef4444' : '#f97316') : '#2a2a2a',
              border: `1px solid ${active ? (warn ? '#ef444435' : '#f9731635') : '#1a1a1a'}`,
              transition: 'all 0.2s',
            }}>{label}</span>
          )
        })}
      </div>
    </div>
  )
}

function BentoCard({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="elec-bento-card">
      <div style={{
        fontSize: 'calc(var(--widget-font-scale, 1) * 9px)', fontWeight: 700, textTransform: 'uppercase',
        letterSpacing: '0.1em', color: '#333', marginBottom: 8,
      }}>{title}</div>
      {children}
    </div>
  )
}

const MGUK_MODES: Record<number, string> = {
  0: 'Off', 1: 'Low', 2: 'Medium', 3: 'High', 4: 'Hot Lap', 5: 'Time Attack',
}

// ─── Main Export ─────────────────────────────────────────────────────────────

export function ElectronicsBento({ snap, fields, fontScale }: Props) {
  const has = (id: string) => fields.some(f => f.id === id)

  const adjFields = fields.filter(f => f.group === 'adjustments')
  const hybridFields = fields.filter(f => f.group === 'hybrid')
  const engineFields = fields.filter(f => f.group === 'engine')
  const hasWarnings = has('engineWarnings')

  const showGear = has('gear')
  const showRpm = has('rpm')
  const showP2p = has('p2p')
  const hasDrs = snap.drsStatus != null
  const hasP2pData = snap.p2pCount != null || snap.p2pStatus != null
  const showHero = showGear || showRpm || hasDrs || (showP2p && hasP2pData) || snap.onPitRoad

  return (
    <div style={{ '--widget-font-scale-local': fontScale } as React.CSSProperties}>
      {showHero && <Hero snap={snap} fontScale={fontScale} showGear={showGear} showRpm={showRpm} showP2p={showP2p} />}

      <div className="elec-bento-grid">
        {/* Driver Adjustments card */}
        {adjFields.length > 0 && (
          <BentoCard title="Adjustments">
            {has('tc1') && snap.dcTractionControl != null && (
              <PillStepper value={snap.dcTractionControl} label="TC 1" color="#22c55e" />
            )}
            {has('tc2') && snap.dcTractionControl2 != null && (
              <PillStepper value={snap.dcTractionControl2} label="TC 2" color="#22c55e" />
            )}
            {has('abs') && snap.dcAbs != null && (
              <PillStepper value={snap.dcAbs} label="ABS" color="#3b82f6" />
            )}
            {has('bb') && snap.brakeBias != null && (
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
                <span style={{ color: '#666', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)' }}>Brake Bias</span>
                <span style={{ color: '#ccc', fontFamily: 'var(--font-mono)', fontSize: 'calc(var(--widget-font-scale, 1) * 12px)' }}>
                  {snap.brakeBias > 1 ? `${snap.brakeBias.toFixed(1)}%` : `${(snap.brakeBias * 100).toFixed(1)}%`}
                </span>
              </div>
            )}
            {adjFields
              .filter(f => !['tc1', 'tc2', 'abs', 'bb'].includes(f.id))
              .map(f => (
                <div key={f.id} style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 3 }}>
                  <span style={{ color: '#666', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)' }}>{f.label}</span>
                  <span style={{ color: '#ccc', fontFamily: 'var(--font-mono)', fontSize: 'calc(var(--widget-font-scale, 1) * 12px)' }}>{f.value}</span>
                </div>
              ))
            }
          </BentoCard>
        )}

        {/* Engine card */}
        {engineFields.length > 0 && (
          <BentoCard title="Engine">
            {has('waterTemp') && <TempBar label="Water" value={snap.waterTemp} low={70} high={105} />}
            {has('oilTemp') && <TempBar label="Oil" value={snap.oilTemp} low={80} high={130} />}
            {has('oilPress') && (
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
                <span style={{ color: '#666', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)' }}>Oil Press</span>
                <span style={{
                  color: snap.oilPress < 2.5 ? '#ef4444' : '#ccc',
                  fontFamily: 'var(--font-mono)', fontSize: 'calc(var(--widget-font-scale, 1) * 12px)',
                }}>
                  {snap.oilPress.toFixed(2)}<span style={{ color: '#555', fontSize: 'calc(var(--widget-font-scale, 1) * 10px)' }}> bar</span>
                </span>
              </div>
            )}
            {has('voltage') && (
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
                <span style={{ color: '#666', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)' }}>Voltage</span>
                <span style={{
                  color: snap.voltage < 12 ? '#ef4444' : snap.voltage < 13 ? '#facc15' : '#22c55e',
                  fontFamily: 'var(--font-mono)', fontSize: 'calc(var(--widget-font-scale, 1) * 12px)',
                }}>
                  {snap.voltage.toFixed(1)}<span style={{ color: '#555', fontSize: 'calc(var(--widget-font-scale, 1) * 10px)' }}> V</span>
                </span>
              </div>
            )}
            {has('fuelPress') && snap.fuelPress != null && (
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
                <span style={{ color: '#666', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)' }}>Fuel Press</span>
                <span style={{ color: '#ccc', fontFamily: 'var(--font-mono)', fontSize: 'calc(var(--widget-font-scale, 1) * 12px)' }}>
                  {snap.fuelPress.toFixed(2)}<span style={{ color: '#555', fontSize: 'calc(var(--widget-font-scale, 1) * 10px)' }}> bar</span>
                </span>
              </div>
            )}
            {has('manifoldPress') && snap.manifoldPress != null && (
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
                <span style={{ color: '#666', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)' }}>Boost</span>
                <span style={{ color: '#3b82f6', fontFamily: 'var(--font-mono)', fontSize: 'calc(var(--widget-font-scale, 1) * 12px)' }}>
                  {snap.manifoldPress.toFixed(2)}<span style={{ color: '#555', fontSize: 'calc(var(--widget-font-scale, 1) * 10px)' }}> bar</span>
                </span>
              </div>
            )}
            {has('speed') && (
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
                <span style={{ color: '#666', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)' }}>Speed</span>
                <span style={{ color: '#ccc', fontFamily: 'var(--font-mono)', fontSize: 'calc(var(--widget-font-scale, 1) * 12px)' }}>
                  {(snap.speedMs * 3.6).toFixed(0)}<span style={{ color: '#555', fontSize: 'calc(var(--widget-font-scale, 1) * 10px)' }}> km/h</span>
                </span>
              </div>
            )}
          </BentoCard>
        )}

        {/* Hybrid / Boost card */}
        {hybridFields.length > 0 && (
          <BentoCard title="Hybrid / Boost">
            {has('battery') && snap.energyBatteryPct != null && (() => {
              const pct = snap.energyBatteryPct
              const color = pct > 0.3 ? '#3b82f6' : pct > 0.15 ? '#facc15' : '#ef4444'
              return (
                <div style={{ marginBottom: 6 }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 2 }}>
                    <span style={{ color: '#666', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)' }}>Battery</span>
                    <span style={{ color, fontFamily: 'var(--font-mono)', fontSize: 'calc(var(--widget-font-scale, 1) * 12px)' }}>{(pct * 100).toFixed(0)}%</span>
                  </div>
                  <div style={{ height: 4, background: '#1a1a1a', borderRadius: 2 }}>
                    <div style={{ height: '100%', width: `${Math.min(pct, 1) * 100}%`, background: color, borderRadius: 2, transition: 'width 0.4s' }} />
                  </div>
                </div>
              )
            })()}
            {has('mgukMode') && snap.mgukDeployMode != null && (
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 5 }}>
                <span style={{ color: '#666', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)' }}>MGU-K Mode</span>
                <span style={{
                  fontSize: 'calc(var(--widget-font-scale, 1) * 10px)', fontFamily: 'var(--font-mono)', fontWeight: 600,
                  padding: '2px 7px', borderRadius: 10,
                  background: '#3b82f620', color: '#3b82f6',
                  border: '1px solid #3b82f640',
                }}>
                  {MGUK_MODES[snap.mgukDeployMode] ?? `Mode ${snap.mgukDeployMode}`}
                </span>
              </div>
            )}
            {has('mgukPower') && snap.powerMguk != null && snap.powerMguk > 0 && (
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
                <span style={{ color: '#666', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)' }}>MGU-K Power</span>
                <span style={{ color: '#3b82f6', fontFamily: 'var(--font-mono)', fontSize: 'calc(var(--widget-font-scale, 1) * 12px)' }}>
                  {(snap.powerMguk / 1000).toFixed(1)}<span style={{ color: '#555', fontSize: 'calc(var(--widget-font-scale, 1) * 10px)' }}> kW</span>
                </span>
              </div>
            )}
            {has('drs') && snap.drsStatus != null && (() => {
              const active = snap.drsStatus >= 2
              const avail = snap.drsStatus >= 1
              return (
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
                  <span style={{ color: '#666', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)' }}>DRS</span>
                  <span className={`elec-led ${active ? 'elec-led-active-green' : avail ? 'elec-led-ready' : ''}`}
                    style={{ fontSize: 'calc(var(--widget-font-scale, 1) * 10px)' }}>
                    {active ? 'ACTIVE' : avail ? 'READY' : 'OFF'}
                  </span>
                </div>
              )
            })()}
            {has('p2p') && (snap.p2pCount != null || snap.p2pStatus != null) && (
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
                <span style={{ color: '#666', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)' }}>Push-to-Pass</span>
                <span className={`elec-led ${snap.p2pStatus ? 'elec-led-active-blue' : ''}`}
                  style={{ fontSize: 'calc(var(--widget-font-scale, 1) * 10px)' }}>
                  {snap.p2pStatus ? 'ACTIVE' : 'OFF'}{snap.p2pCount != null ? ` ×${snap.p2pCount}` : ''}
                </span>
              </div>
            )}
          </BentoCard>
        )}

        {/* Status card */}
        {hasWarnings && (
          <BentoCard title="Status">
            <EngineWarningsLeds warnings={snap.engineWarnings} />
            {has('shiftLights') && snap.shiftLightShiftRpm != null && (
              <div style={{ marginTop: 8 }}>
                <span style={{ color: '#444', fontSize: 'calc(var(--widget-font-scale, 1) * 10px)', textTransform: 'uppercase', letterSpacing: '0.06em', display: 'block', marginBottom: 4 }}>
                  Shift Lights
                </span>
                <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
                  {snap.shiftLightFirstRpm != null && (
                    <span style={{ color: '#555', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)', fontFamily: 'var(--font-mono)' }}>
                      {snap.shiftLightFirstRpm.toFixed(0)}
                    </span>
                  )}
                  <span style={{ color: '#facc15', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)', fontFamily: 'var(--font-mono)' }}>
                    ↑{snap.shiftLightShiftRpm.toFixed(0)}
                  </span>
                  {snap.shiftLightBlinkRpm != null && (
                    <span style={{ color: '#ef4444', fontSize: 'calc(var(--widget-font-scale, 1) * 11px)', fontFamily: 'var(--font-mono)' }}>
                      ✦{snap.shiftLightBlinkRpm.toFixed(0)}
                    </span>
                  )}
                </div>
              </div>
            )}
          </BentoCard>
        )}
      </div>
    </div>
  )
}
