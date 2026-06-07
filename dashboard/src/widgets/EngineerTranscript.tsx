import { useEffect, useRef, useState } from 'react'
import { engineerService } from '../features/race-engineer/audio/AudioEngineerService'
import type { EngineerAudioMsg } from '../features/race-engineer/types'
import { SettingsDrawer } from '../components/SettingsDrawer'
import { EngineerTranscriptSettings } from './EngineerTranscriptSettings'

// ─── Types ────────────────────────────────────────────────────────────────────

interface TranscriptEntry {
  seq: number
  ts: number
  text: string
  priority: EngineerAudioMsg['priority']
  requestId: string
}

// ─── Persistence ──────────────────────────────────────────────────────────────

const PREFS_KEY = 'iracing-engineer-transcript-prefs-v1'

function loadPrefs(): { fontScale: number } {
  try {
    const raw = localStorage.getItem(PREFS_KEY)
    if (raw) {
      const p = JSON.parse(raw)
      return { fontScale: typeof p.fontScale === 'number' ? p.fontScale : 1 }
    }
  } catch { /* ignore */ }
  return { fontScale: 1 }
}

function savePrefs(fontScale: number) {
  try { localStorage.setItem(PREFS_KEY, JSON.stringify({ fontScale })) } catch { /* ignore */ }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

function fmtTime(ts: number): string {
  const d = new Date(ts)
  const hh = String(d.getHours()).padStart(2, '0')
  const mm = String(d.getMinutes()).padStart(2, '0')
  const ss = String(d.getSeconds()).padStart(2, '0')
  return `${hh}:${mm}:${ss}`
}

const PRIORITY_COLOR: Record<EngineerAudioMsg['priority'], string> = {
  critical: '#f87171',
  high: '#fbbf24',
  info: '#9ca3af',
}

const PRIORITY_LABEL: Record<EngineerAudioMsg['priority'], string> = {
  critical: 'CRIT',
  high: 'HIGH',
  info: 'INFO',
}

const MAX_ENTRIES = 100

// ─── Component ────────────────────────────────────────────────────────────────

export function EngineerTranscript() {
  const [entries, setEntries] = useState<TranscriptEntry[]>([])
  const seqRef = useRef(0)
  const [showSettings, setShowSettings] = useState(false)
  const [fontScale, setFontScale] = useState(() => loadPrefs().fontScale)

  useEffect(() => {
    const off = engineerService.onAudioMsg((msg) => {
      const entry: TranscriptEntry = {
        seq: seqRef.current++,
        ts: Date.now(),
        text: msg.text,
        priority: msg.priority,
        requestId: msg.requestId,
      }
      setEntries((prev) => [entry, ...prev].slice(0, MAX_ENTRIES))
    })
    return () => { off() }
  }, [])

  function handleFontScale(s: number) { setFontScale(s); savePrefs(s) }
  function handleResetAll() { setFontScale(1); savePrefs(1) }

  return (
    <section className="card" style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <h2 style={{ display: 'flex', alignItems: 'center' }}>
        <span>Engineer Transcript</span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginLeft: 'auto' }}>
          {entries.length > 0 && (
            <button
              onClick={() => setEntries([])}
              style={{
                background: 'none',
                border: 'none',
                color: '#6b7280',
                fontSize: 11,
                cursor: 'pointer',
                padding: '0 4px',
                lineHeight: 1,
              }}
              title="Clear log"
            >
              Clear
            </button>
          )}
          <button
            className={`header-btn${showSettings ? ' header-btn-active' : ''}`}
            onClick={() => setShowSettings(s => !s)}
          >⚙</button>
        </div>
      </h2>

      <SettingsDrawer open={showSettings} onClose={() => setShowSettings(false)} title="Engineer Transcript Settings">
        <EngineerTranscriptSettings
          fontScale={fontScale}
          onFontScale={handleFontScale}
          onResetAll={handleResetAll}
        />
      </SettingsDrawer>

      <div
        className="card-body"
        style={{
          flex: 1,
          overflowY: 'auto',
          lineHeight: 1.5,
          padding: '4px 0',
        }}
      >
        {entries.length === 0 ? (
          <div style={{ color: '#4b5563', padding: '8px 12px', lineHeight: 1.6 }}>
            <div style={{ marginBottom: 6, fontSize: `calc(12px * ${fontScale})` }}>
              No callouts yet — the engineer hasn't spoken.
            </div>
            <div style={{ fontSize: `calc(11px * ${fontScale})` }}>
              If the widget stays empty during a session, the backend isn't emitting audio
              (check: engineer enabled, voice installed &amp; selected).
              If text appears here but you hear nothing, it's an audio playback issue.
            </div>
          </div>
        ) : (
          entries.map((e) => (
            <div
              key={e.seq}
              style={{
                display: 'flex',
                alignItems: 'baseline',
                gap: 8,
                padding: '4px 12px',
                borderBottom: '1px solid #1f1f1f',
              }}
            >
              <span style={{
                color: '#4b5563',
                fontVariantNumeric: 'tabular-nums',
                flexShrink: 0,
                fontSize: `calc(11px * ${fontScale})`,
              }}>
                {fmtTime(e.ts)}
              </span>
              <span
                style={{
                  color: PRIORITY_COLOR[e.priority],
                  fontSize: `calc(10px * ${fontScale})`,
                  fontWeight: 700,
                  letterSpacing: '0.04em',
                  flexShrink: 0,
                  minWidth: 28,
                }}
              >
                {PRIORITY_LABEL[e.priority]}
              </span>
              <span style={{ color: '#e5e7eb', fontSize: `calc(12px * ${fontScale})` }}>
                {e.text}
              </span>
            </div>
          ))
        )}
      </div>
    </section>
  )
}
