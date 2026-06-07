import { useState } from 'react'
import type { VoiceInfo } from '../types'
import { engineerService } from '../audio/AudioEngineerService'
import { VoiceCard } from './VoiceCard'

type Step = 'piper' | 'voice' | 'radio-check'

interface Props {
  initialStep?: Step
  initialVoices: VoiceInfo[]
  initialVoiceId?: string | null
  onComplete: (voiceId: string) => void
  onSkip?: () => void
}

const STEPS: { key: Step; label: string }[] = [
  { key: 'piper', label: 'Install Piper' },
  { key: 'voice', label: 'Choose Voice' },
  { key: 'radio-check', label: 'Radio Check' },
]

export function SetupWizard({
  initialStep = 'piper',
  initialVoices,
  initialVoiceId = null,
  onComplete,
  onSkip,
}: Props) {
  const [step, setStep] = useState<Step>(initialStep)
  const [voices, setVoices] = useState<VoiceInfo[]>(initialVoices)
  const [selectedVoiceId, setSelectedVoiceId] = useState<string | null>(initialVoiceId)
  const [installing, setInstalling] = useState(false)
  const [installError, setInstallError] = useState<string | null>(null)

  function handleVoiceInstalled(voiceId: string) {
    setVoices((prev) =>
      prev.map((v) => (v.id === voiceId ? { ...v, installed: true } : v)),
    )
    setSelectedVoiceId(voiceId)
    setStep('radio-check')
  }

  async function handleInstallPiper() {
    setInstalling(true)
    setInstallError(null)

    const onComplete = (msg: { target: string; success: boolean; error: string | null }) => {
      if (msg.target !== 'piper') return
      setInstalling(false)
      if (msg.success) {
        setStep('voice')
      } else {
        setInstallError(msg.error ?? 'Installation failed. Check your internet connection.')
      }
    }

    const off = engineerService.onInstallComplete(onComplete)
    engineerService.installPiper()
    // Cleanup after 60s timeout
    setTimeout(() => { off(); setInstalling(false) }, 60000)
  }

  const stepIdx = STEPS.findIndex((s) => s.key === step)

  // When resuming after a crash/close without finishing, selectedVoiceId may be null
  // (it was only in-memory React state). Fall back to the first installed voice so the
  // Finish button is always usable when installation is complete.
  const effectiveVoiceId =
    (selectedVoiceId && voices.some((v) => v.id === selectedVoiceId && v.installed))
      ? selectedVoiceId
      : voices.find((v) => v.installed)?.id ?? null

  return (
    <div style={{ padding: '0 4px' }}>
      {/* Step indicator */}
      <div style={{ display: 'flex', gap: 4, marginBottom: 20 }}>
        {STEPS.map((s, i) => (
          <div
            key={s.key}
            style={{
              flex: 1,
              textAlign: 'center',
              fontSize: 11,
              color: i === stepIdx ? '#6366f1' : '#6b7280',
              borderBottom: `2px solid ${i === stepIdx ? '#6366f1' : '#333'}`,
              paddingBottom: 6,
            }}
          >
            {s.label}
          </div>
        ))}
      </div>

      {/* Step content */}
      {step === 'piper' && (
        <div>
          <p style={{ color: '#9ca3af', fontSize: 13, lineHeight: 1.5, marginBottom: 16 }}>
            The Race Engineer uses <strong style={{ color: '#e5e7eb' }}>Piper TTS</strong> — a
            local neural text-to-speech engine. It runs entirely on your machine with no internet
            connection needed after setup.
          </p>
          <p style={{ color: '#6b7280', fontSize: 12, marginBottom: 20 }}>
            Piper (~30 MB) will be downloaded once and stored in your AppData folder.
          </p>
          {installError && (
            <div style={{ color: '#f87171', fontSize: 12, marginBottom: 12 }}>{installError}</div>
          )}
          <button
            className="header-btn"
            onClick={handleInstallPiper}
            disabled={installing}
            style={{ width: '100%', fontSize: 13 }}
          >
            {installing ? 'Installing Piper…' : 'Install Piper TTS'}
          </button>
          {onSkip && (
            <button
              className="header-btn"
              onClick={onSkip}
              style={{ width: '100%', marginTop: 8, fontSize: 12, color: '#6b7280' }}
            >
              Skip setup
            </button>
          )}
        </div>
      )}

      {step === 'voice' && (
        <div>
          <p style={{ color: '#9ca3af', fontSize: 13, marginBottom: 16 }}>
            Choose a voice for your engineer. You can install more voices later.
          </p>
          <div style={{ maxHeight: 320, overflowY: 'auto' }}>
            {voices.map((v) => (
              <VoiceCard
                key={v.id}
                voice={v}
                isActive={v.id === selectedVoiceId}
                onActivate={setSelectedVoiceId}
                onUninstalled={() => {}}
                onInstalled={handleVoiceInstalled}
              />
            ))}
          </div>
        </div>
      )}

      {step === 'radio-check' && (
        <div>
          <p style={{ color: '#9ca3af', fontSize: 13, marginBottom: 16 }}>
            Your engineer is ready. Click the radio check button on the voice card below to
            hear a sample.
          </p>
          <div>
            {voices
              .filter((v) => v.id === effectiveVoiceId && v.installed)
              .map((v) => (
                <VoiceCard
                  key={v.id}
                  voice={v}
                  isActive={true}
                  onActivate={() => {}}
                  onUninstalled={() => {}}
                  onInstalled={() => {}}
                />
              ))}
          </div>
          <button
            className="header-btn"
            onClick={() => effectiveVoiceId && onComplete(effectiveVoiceId)}
            disabled={!effectiveVoiceId}
            style={{ width: '100%', marginTop: 12, fontSize: 13 }}
          >
            Finish Setup
          </button>
        </div>
      )}
    </div>
  )
}
