import { useEffect, useRef, useState } from 'react'
import type { VoiceInfo } from '../types'
import { engineerService } from '../audio/AudioEngineerService'

interface Props {
  voice: VoiceInfo
  isActive: boolean
  onActivate: (voiceId: string) => void
  onUninstalled: (voiceId: string) => void
  onInstalled: (voiceId: string) => void
}

const RADIO_CHECK_PHRASE =
  'Radio check, radio check. Box this lap, box this lap. Push now, you have good pace.'

export function VoiceCard({ voice, isActive, onActivate, onUninstalled, onInstalled }: Props) {
  const [progress, setProgress] = useState<number | null>(null)
  const [stage, setStage] = useState<string>('')
  const [error, setError] = useState<string | null>(null)
  const [synthPending, setSynthPending] = useState(false)
  const [previewing, setPreviewing] = useState(false)
  const synthReqId = useRef<string | null>(null)
  const previewAudio = useRef<HTMLAudioElement | null>(null)

  useEffect(() => {
    const offProgress = engineerService.onInstallProgress((msg) => {
      if (msg.targetId !== voice.id) return
      const pct =
        msg.bytesTotal && msg.bytesTotal > 0
          ? Math.round((msg.bytesDownloaded / msg.bytesTotal) * 100)
          : null
      setProgress(pct)
      setStage(msg.stage)
    })

    const offComplete = engineerService.onInstallComplete((msg) => {
      if (msg.targetId !== voice.id) return
      setProgress(null)
      setStage('')
      if (msg.success) {
        setError(null)
        onInstalled(voice.id)
      } else {
        setError(msg.error ?? 'Installation failed')
      }
    })

    const offAudio = engineerService.onAudioMsg((msg) => {
      if (msg.requestId === synthReqId.current) {
        setSynthPending(false)
        synthReqId.current = null
      }
    })

    return () => {
      offProgress(); offComplete(); offAudio()
      previewAudio.current?.pause()
      previewAudio.current = null
    }
  }, [voice.id, onInstalled])

  function handlePreview() {
    if (previewing) {
      previewAudio.current?.pause()
      previewAudio.current = null
      setPreviewing(false)
      return
    }
    const audio = new Audio(voice.sampleUrl)
    previewAudio.current = audio
    setPreviewing(true)
    audio.play().catch(() => setPreviewing(false))
    audio.onended = () => { setPreviewing(false); previewAudio.current = null }
    audio.onerror = () => { setPreviewing(false); previewAudio.current = null }
  }

  function handleInstall() {
    setError(null)
    setProgress(0)
    setStage('downloading')
    engineerService.installVoice(voice.id)
  }

  function handleUninstall() {
    engineerService.uninstallVoice(voice.id)
    onUninstalled(voice.id)
  }

  function handleRadioCheck() {
    if (synthPending) return
    const reqId = `radio-check-${voice.id}-${Date.now()}`
    synthReqId.current = reqId
    setSynthPending(true)
    engineerService.synthesize(voice.id, RADIO_CHECK_PHRASE, reqId)
  }

  const isInstalling = progress !== null

  return (
    <div
      style={{
        border: `1px solid ${isActive ? '#6366f1' : '#333'}`,
        borderRadius: 8,
        padding: '12px 14px',
        background: isActive ? 'rgba(99,102,241,0.08)' : '#1a1a1a',
        marginBottom: 10,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 4 }}>
        <span style={{ fontSize: 18 }}>{voice.language.startsWith('en-GB') ? '🇬🇧' : '🇺🇸'}</span>
        <span style={{ color: '#e5e7eb', fontWeight: 600, fontSize: 14 }}>{voice.name}</span>
        <span style={{ color: '#6b7280', fontSize: 12, marginLeft: 'auto' }}>
          {voice.sizeMb} MB
        </span>
      </div>
      <div style={{ color: '#9ca3af', fontSize: 12, marginBottom: 10 }}>{voice.description}</div>

      {isInstalling && (
        <div style={{ marginBottom: 8 }}>
          <div style={{ color: '#9ca3af', fontSize: 11, marginBottom: 3 }}>
            {stage}{progress !== null ? ` ${progress}%` : ''}
          </div>
          <div style={{ background: '#333', borderRadius: 4, height: 4 }}>
            <div
              style={{
                background: '#6366f1',
                borderRadius: 4,
                height: 4,
                width: `${progress ?? 0}%`,
                transition: 'width 0.2s',
              }}
            />
          </div>
        </div>
      )}

      {error && (
        <div style={{ color: '#f87171', fontSize: 11, marginBottom: 8 }}>{error}</div>
      )}

      <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
        <button
          className="header-btn"
          onClick={handlePreview}
          style={{ fontSize: 12, minWidth: 28 }}
          title={previewing ? 'Stop preview' : 'Preview voice sample'}
        >
          {previewing ? '⏹' : '▶'}
        </button>
        {!voice.installed && !isInstalling && (
          <button className="header-btn" onClick={handleInstall} style={{ fontSize: 12 }}>
            Install ({voice.sizeMb} MB)
          </button>
        )}
        {voice.installed && !isActive && (
          <button
            className="header-btn"
            onClick={() => onActivate(voice.id)}
            style={{ fontSize: 12 }}
          >
            Use this voice
          </button>
        )}
        {voice.installed && isActive && (
          <span
            style={{
              fontSize: 12,
              color: '#6366f1',
              border: '1px solid #6366f1',
              borderRadius: 4,
              padding: '2px 8px',
            }}
          >
            ✓ Active
          </span>
        )}
        {voice.installed && (
          <button
            className="header-btn"
            onClick={handleRadioCheck}
            disabled={synthPending}
            style={{ fontSize: 12 }}
          >
            {synthPending ? 'Speaking…' : 'Radio check'}
          </button>
        )}
        {voice.installed && (
          <button
            className="header-btn header-btn-danger"
            onClick={handleUninstall}
            style={{ fontSize: 12 }}
          >
            Uninstall
          </button>
        )}
      </div>
    </div>
  )
}
