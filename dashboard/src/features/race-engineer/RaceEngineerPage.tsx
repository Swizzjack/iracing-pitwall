import { useEffect, useState } from 'react'
import type { VoiceInfo } from './types'
import type { EngineerSettings } from './state/engineerSettings'
import {
  isSetupCompleted,
  markSetupCompleted,
  saveEngineerSettings,
} from './state/engineerSettings'
import { engineerService } from './audio/AudioEngineerService'
import { SetupWizard } from './components/SetupWizard'
import { SettingsPanel } from './components/SettingsPanel'

interface Props {
  settings: EngineerSettings
  onSettingsChange: (partial: Partial<EngineerSettings>) => void
  onClose: () => void
}

export function RaceEngineerPage({ settings, onSettingsChange, onClose }: Props) {
  const [loading, setLoading] = useState(true)
  const [piperInstalled, setPiperInstalled] = useState(false)
  const [voices, setVoices] = useState<VoiceInfo[]>([])
  const [showWizard, setShowWizard] = useState(!isSetupCompleted())

  useEffect(() => {
    const off = engineerService.onStatusChange((status) => {
      if (status.kind === 'connected') {
        setPiperInstalled(status.piperInstalled)
        setVoices(status.voices)
        setLoading(false)
      }
    })
    // Request fresh status on mount
    engineerService.requestStatus()
    return () => { off() }
  }, [])

  function handleWizardComplete(voiceId: string) {
    markSetupCompleted()
    onSettingsChange({ activeVoiceId: voiceId, enabled: true })
    saveEngineerSettings({ ...settings, activeVoiceId: voiceId, enabled: true })
    setShowWizard(false)
  }

  function handleResetSetup() {
    setShowWizard(true)
  }

  const initialWizardStep = piperInstalled
    ? voices.some((v) => v.installed)
      ? 'radio-check'
      : 'voice'
    : 'piper'

  return (
    <div>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          paddingBottom: 12,
          borderBottom: '1px solid #222',
          marginBottom: 16,
        }}
      >
        <span style={{ color: '#e5e7eb', fontWeight: 600, fontSize: 14 }}>
          🎙 Race Engineer
        </span>
        <button className="settings-drawer-close" onClick={onClose} title="Close">
          ✕
        </button>
      </div>

      {loading ? (
        <div style={{ color: '#6b7280', fontSize: 13, textAlign: 'center', padding: 24 }}>
          Connecting to bridge…
        </div>
      ) : showWizard ? (
        <SetupWizard
          initialStep={initialWizardStep as 'piper' | 'voice' | 'radio-check'}
          initialVoices={voices}
          initialVoiceId={settings.activeVoiceId ?? voices.find((v) => v.installed)?.id ?? null}
          onComplete={handleWizardComplete}
          onSkip={() => {
            markSetupCompleted()
            setShowWizard(false)
          }}
        />
      ) : (
        <SettingsPanel
          settings={settings}
          voices={voices}
          onSettingsChange={onSettingsChange}
          onResetSetup={handleResetSetup}
        />
      )}
    </div>
  )
}
