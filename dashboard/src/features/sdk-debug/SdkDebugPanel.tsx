import { useEffect, useMemo, useState, type CSSProperties, type ReactNode } from 'react'
import type { SdkDebugSnapshot } from '@shared/SdkDebugSnapshot'
import type { ClientMessage } from '@shared/ClientMessage'

interface Props {
  snapshot: SdkDebugSnapshot | null
  send: (msg: ClientMessage) => void
  onClose: () => void
}

/**
 * Hidden admin/debug overlay — live dump of EVERYTHING the iRacing SDK
 * exposes: every telemetry variable (with current value, unit, type,
 * description), the full raw session-info YAML and header diagnostics.
 *
 * Nothing here is persisted; it's a pure live view for the operator to
 * inspect what data is actually available from the SDK. The feed is gated
 * on the bridge — it only runs while this panel is mounted (see effect below).
 */
export function SdkDebugPanel({ snapshot, send, onClose }: Props) {
  const [filter, setFilter] = useState('')
  const [showYaml, setShowYaml] = useState(false)

  // Tell the bridge to start/stop building the full SDK dump. It's somewhat
  // expensive (walks every SDK variable every tick), so it only runs while
  // this panel is actually open.
  useEffect(() => {
    send({ type: 'setSdkDebug', enabled: true })
    return () => send({ type: 'setSdkDebug', enabled: false })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const filteredVars = useMemo(() => {
    if (!snapshot) return []
    const q = filter.trim().toLowerCase()
    if (!q) return snapshot.vars
    return snapshot.vars.filter(
      (v) => v.name.toLowerCase().includes(q) || v.desc.toLowerCase().includes(q),
    )
  }, [snapshot, filter])

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        background: '#0a0a0a',
        zIndex: 300,
        display: 'flex',
        flexDirection: 'column',
        color: '#e5e7eb',
        fontFamily: 'monospace',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '12px 16px',
          borderBottom: '1px solid #222',
          flexShrink: 0,
        }}
      >
        <span style={{ fontWeight: 600, fontSize: 14 }}>
          🐞 SDK Admin / Debug View — live, nothing is stored
        </span>
        <button className="settings-drawer-close" onClick={onClose} title="Close">✕</button>
      </div>

      {!snapshot ? (
        <div style={{ color: '#6b7280', fontSize: 13, textAlign: 'center', padding: 24 }}>
          Waiting for first SDK dump… (requires iRacing to be running)
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', flex: 1, minHeight: 0, padding: '12px 16px', gap: 12 }}>
          {/* Header diagnostics */}
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px 20px', fontSize: 12, color: '#9ca3af' }}>
            <DiagField label="connected" value={String(snapshot.header.connected)} />
            <DiagField label="ver" value={String(snapshot.header.ver)} />
            <DiagField label="tickRate" value={`${snapshot.header.tickRate} Hz`} />
            <DiagField label="numVars" value={String(snapshot.header.numVars)} />
            <DiagField label="numBuf" value={String(snapshot.header.numBuf)} />
            <DiagField label="bufLen" value={`${snapshot.header.bufLen} B`} />
            <DiagField label="sessionInfoUpdate" value={String(snapshot.header.sessionInfoUpdate)} />
          </div>

          {/* Search */}
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <input
              type="text"
              placeholder="Filter by name or description…"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              style={{
                flex: 1,
                maxWidth: 420,
                background: '#171717',
                border: '1px solid #333',
                borderRadius: 4,
                color: '#e5e7eb',
                padding: '6px 10px',
                fontSize: 12,
                fontFamily: 'monospace',
              }}
            />
            <span style={{ color: '#6b7280', fontSize: 12 }}>
              {filteredVars.length} / {snapshot.vars.length} variables
            </span>
          </div>

          {/* Variable table */}
          <div style={{ flex: 1, minHeight: 0, overflow: 'auto', border: '1px solid #222', borderRadius: 4 }}>
            <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 12 }}>
              <thead>
                <tr style={{ position: 'sticky', top: 0, background: '#171717', textAlign: 'left' }}>
                  <Th>Name</Th>
                  <Th>Value</Th>
                  <Th>Unit</Th>
                  <Th>Type</Th>
                  <Th>#</Th>
                  <Th>Description</Th>
                </tr>
              </thead>
              <tbody>
                {filteredVars.map((v) => (
                  <tr key={v.name} style={{ borderTop: '1px solid #1a1a1a' }}>
                    <Td mono style={{ color: '#e5e7eb', whiteSpace: 'nowrap' }}>{v.name}</Td>
                    <Td mono style={{ color: '#facc15', maxWidth: 360, overflow: 'hidden', textOverflow: 'ellipsis' }}>
                      {v.values.length === 0 ? (
                        <span style={{ color: '#6b7280' }}>—</span>
                      ) : v.values.length === 1 ? (
                        v.values[0]
                      ) : (
                        `[${v.values.join(', ')}]`
                      )}
                    </Td>
                    <Td style={{ color: '#9ca3af', whiteSpace: 'nowrap' }}>{v.unit || '—'}</Td>
                    <Td style={{ color: '#9ca3af', whiteSpace: 'nowrap' }}>{v.varType}</Td>
                    <Td style={{ color: '#9ca3af' }}>{v.count}</Td>
                    <Td style={{ color: '#6b7280' }}>{v.desc}</Td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {/* Raw session YAML */}
          <div style={{ flexShrink: 0, maxHeight: showYaml ? '40vh' : 'auto', display: 'flex', flexDirection: 'column' }}>
            <button
              className="header-btn"
              style={{ width: 'fit-content', fontSize: 12 }}
              onClick={() => setShowYaml((v) => !v)}
            >
              {showYaml ? '▾' : '▸'} Raw session-info YAML (unfiltered, {snapshot.sessionYamlRaw.length.toLocaleString()} chars)
            </button>
            {showYaml && (
              <pre
                style={{
                  marginTop: 8,
                  flex: 1,
                  overflow: 'auto',
                  background: '#111',
                  border: '1px solid #222',
                  borderRadius: 4,
                  padding: 10,
                  fontSize: 11,
                  lineHeight: 1.4,
                  color: '#9ca3af',
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-word',
                }}
              >
                {snapshot.sessionYamlRaw}
              </pre>
            )}
          </div>

          <div style={{ color: '#444', fontSize: 11 }}>
            Live view only — nothing on this screen is written to disk or sent anywhere besides your browser.
          </div>
        </div>
      )}
    </div>
  )
}

function DiagField({ label, value }: { label: string; value: string }) {
  return (
    <span>
      <span style={{ color: '#6b7280' }}>{label}=</span>
      <span style={{ color: '#e5e7eb' }}>{value}</span>
    </span>
  )
}

function Th({ children }: { children: ReactNode }) {
  return (
    <th style={{ padding: '6px 10px', fontWeight: 600, color: '#9ca3af', borderBottom: '1px solid #222' }}>
      {children}
    </th>
  )
}

function Td({ children, mono, style }: { children: ReactNode; mono?: boolean; style?: CSSProperties }) {
  return (
    <td style={{ padding: '4px 10px', fontFamily: mono ? 'monospace' : undefined, ...style }}>
      {children}
    </td>
  )
}
