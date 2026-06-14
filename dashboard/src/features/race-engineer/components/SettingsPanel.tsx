import { useEffect, useRef, useState } from 'react'
import type { VoiceInfo } from '../types'
import type { EngineerSettings } from '../state/engineerSettings'
import { resetSetup } from '../state/engineerSettings'
import { VoiceCard } from './VoiceCard'

interface Props {
  settings: EngineerSettings
  voices: VoiceInfo[]
  onSettingsChange: (s: Partial<EngineerSettings>) => void
  onResetSetup: () => void
}

export function SettingsPanel({ settings, voices, onSettingsChange, onResetSetup }: Props) {
  // Optimistic install-state overrides: VoiceCard reports install/uninstall
  // completion slightly before the next EngineerStatus refresh arrives. The
  // server list (`voices`) stays the source of truth; deriving instead of
  // mirroring it into state avoids the prop→state sync effect.
  const [installedOverrides, setInstalledOverrides] = useState<Record<string, boolean>>({})
  const voiceList = voices.map((v) =>
    installedOverrides[v.id] != null && installedOverrides[v.id] !== v.installed
      ? { ...v, installed: installedOverrides[v.id] }
      : v,
  )

  // Audio output device enumeration ------------------------------------------------
  const [audioDevices, setAudioDevices] = useState<MediaDeviceInfo[]>([])
  const mountedRef = useRef(true)
  const deviceSupported =
    typeof navigator !== 'undefined' &&
    !!navigator.mediaDevices?.enumerateDevices &&
    'setSinkId' in AudioContext.prototype

  useEffect(() => {
    mountedRef.current = true
    if (!deviceSupported) return

    async function loadDevices() {
      let devices = await navigator.mediaDevices.enumerateDevices()
      // Labels are empty until audio permission is granted. Request minimal
      // microphone access once to unlock labels, then immediately stop the stream.
      if (devices.some((d) => d.kind === 'audiooutput' && !d.label)) {
        try {
          const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
          stream.getTracks().forEach((t) => t.stop())
          devices = await navigator.mediaDevices.enumerateDevices()
        } catch { /* permission denied — labels stay empty, that's fine */ }
      }
      if (!mountedRef.current) return
      setAudioDevices(devices.filter((d) => d.kind === 'audiooutput'))
    }

    void loadDevices()

    const handler = () => { void loadDevices() }
    navigator.mediaDevices.addEventListener('devicechange', handler)
    return () => {
      mountedRef.current = false
      navigator.mediaDevices.removeEventListener('devicechange', handler)
    }
  }, [deviceSupported])
  // ---------------------------------------------------------------------------------

  function handleUninstalled(voiceId: string) {
    setInstalledOverrides((prev) => ({ ...prev, [voiceId]: false }))
    if (settings.activeVoiceId === voiceId) {
      onSettingsChange({ activeVoiceId: null, enabled: false })
    }
  }

  function handleInstalled(voiceId: string) {
    setInstalledOverrides((prev) => ({ ...prev, [voiceId]: true }))
    onSettingsChange({ activeVoiceId: voiceId })
  }

  function handleActivate(voiceId: string) {
    onSettingsChange({ activeVoiceId: voiceId })
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 0 }}>
      {/* Enable / disable */}
      <div className="settings-section">
        <div className="settings-section-title">Race Engineer</div>
        <div className="settings-footer-row">
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Enabled</label>
          <input
            type="checkbox"
            checked={settings.enabled}
            disabled={!settings.activeVoiceId}
            onChange={(e) => onSettingsChange({ enabled: e.target.checked })}
          />
        </div>
        {!settings.activeVoiceId && (
          <div style={{ color: '#6b7280', fontSize: 11, marginTop: 4 }}>
            Install and select a voice to enable the engineer.
          </div>
        )}
      </div>

      {/* Volume */}
      <div className="settings-section">
        <div className="settings-section-title">Audio</div>
        <div className="settings-footer-row">
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Volume</label>
          <input
            type="range"
            min={0}
            max={2}
            step={0.05}
            value={settings.volume}
            onChange={(e) => onSettingsChange({ volume: parseFloat(e.target.value) })}
            style={{ flex: 1 }}
          />
          <span style={{ color: '#888', fontSize: 'var(--settings-fs)', minWidth: 36, textAlign: 'right' }}>
            {Math.round(settings.volume * 100)}%
          </span>
        </div>
        <div className="settings-footer-row" style={{ marginTop: 8 }}>
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Radio effect</label>
          <select
            value={settings.radioEffect}
            onChange={(e) =>
              onSettingsChange({ radioEffect: e.target.value as EngineerSettings['radioEffect'] })
            }
            style={{
              background: '#1a1a1a',
              color: '#e5e7eb',
              border: '1px solid #333',
              borderRadius: 4,
              padding: '2px 6px',
              fontSize: 'var(--settings-fs)',
            }}
          >
            <option value="off">Off (clean)</option>
            <option value="subtle">Subtle (recommended)</option>
            <option value="medium">Medium</option>
            <option value="strong">Strong</option>
          </select>
        </div>
        <div className="settings-footer-row" style={{ marginTop: 8 }}>
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Output device</label>
          {deviceSupported ? (
            <select
              value={settings.outputDeviceId ?? ''}
              onChange={(e) =>
                onSettingsChange({ outputDeviceId: e.target.value || null })
              }
              style={{
                background: '#1a1a1a',
                color: '#e5e7eb',
                border: '1px solid #333',
                borderRadius: 4,
                padding: '2px 6px',
                fontSize: 'var(--settings-fs)',
                maxWidth: 180,
              }}
            >
              <option value="">System default</option>
              {audioDevices.map((d, i) => (
                <option key={d.deviceId} value={d.deviceId}>
                  {d.label || `Output ${i + 1}`}
                </option>
              ))}
            </select>
          ) : (
            <span style={{ color: '#6b7280', fontSize: 'var(--settings-fs)' }}>
              Requires localhost + Chrome/Edge
            </span>
          )}
        </div>
      </div>

      {/* Behaviour */}
      <div className="settings-section">
        <div className="settings-section-title">Behaviour</div>
        <div className="settings-footer-row">
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Frequency</label>
          <select
            value={settings.frequency}
            onChange={(e) =>
              onSettingsChange({ frequency: e.target.value as EngineerSettings['frequency'] })
            }
            style={{
              background: '#1a1a1a',
              color: '#e5e7eb',
              border: '1px solid #333',
              borderRadius: 4,
              padding: '2px 6px',
              fontSize: 'var(--settings-fs)',
            }}
          >
            <option value="low">Low (critical only)</option>
            <option value="medium">Medium (recommended)</option>
            <option value="high">High (chatty)</option>
          </select>
        </div>
        <div className="settings-footer-row" style={{ marginTop: 8 }}>
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Mute in qualifying</label>
          <input
            type="checkbox"
            checked={settings.muteInQualifying}
            onChange={(e) => onSettingsChange({ muteInQualifying: e.target.checked })}
          />
        </div>
        <div className="settings-footer-row" style={{ marginTop: 8 }}>
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>
            Force all rules in practice (debug)
          </label>
          <input
            type="checkbox"
            checked={settings.debugAllRulesInPractice}
            onChange={(e) => onSettingsChange({ debugAllRulesInPractice: e.target.checked })}
          />
        </div>
        <div style={{ color: '#555', fontSize: 10, marginTop: 3, lineHeight: 1.4 }}>
          Practice already includes pace and weather callouts. This forces race-only rules too (e.g. gaps), which may be inaccurate in practice.
        </div>
      </div>

      {/* Pilot name */}
      <div className="settings-section">
        <div className="settings-section-title">Pilot Name</div>
        <div className="settings-footer-row">
          <input
            type="text"
            placeholder="e.g. Max"
            value={settings.pilotName}
            maxLength={30}
            onChange={(e) => onSettingsChange({ pilotName: e.target.value })}
            style={{
              flex: 1,
              background: '#1a1a1a',
              color: '#e5e7eb',
              border: '1px solid #333',
              borderRadius: 4,
              padding: '4px 8px',
              fontSize: 'var(--settings-fs)',
            }}
          />
        </div>
        <div className="settings-footer-row" style={{ marginTop: 8 }}>
          <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>
            Mute name in callouts
          </label>
          <input
            type="checkbox"
            checked={settings.muteNameInCallouts}
            onChange={(e) => onSettingsChange({ muteNameInCallouts: e.target.checked })}
          />
        </div>
      </div>

      {/* Voices */}
      <div className="settings-section">
        <div className="settings-section-title">Voices</div>
        <div style={{ maxHeight: 360, overflowY: 'auto' }}>
          {voiceList.map((v) => (
            <VoiceCard
              key={v.id}
              voice={v}
              isActive={v.id === settings.activeVoiceId}
              onActivate={handleActivate}
              onUninstalled={handleUninstalled}
              onInstalled={handleInstalled}
            />
          ))}
        </div>
      </div>

      {/* Reset setup */}
      <div className="settings-section">
        <button
          className="header-btn header-btn-danger"
          style={{ width: '100%', fontSize: 12 }}
          onClick={() => {
            resetSetup()
            onResetSetup()
          }}
        >
          Reset setup wizard
        </button>
      </div>
    </div>
  )
}
