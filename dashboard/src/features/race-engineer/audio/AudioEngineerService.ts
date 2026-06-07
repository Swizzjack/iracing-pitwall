//! Audio playback service for the Race Engineer.
//!
//! This service is a singleton that:
//! - Receives engineer server messages forwarded from the main WsClient
//! - Decodes incoming WAV audio (base64) via Web Audio API
//! - Queues and plays audio clips with priority (critical interrupts)
//! - Applies optional radio effect chain
//! - Sends engineer commands via a callback

import type { RadioEffectMode } from './radioEffect'
import { connectWithEffect } from './radioEffect'
import { PriorityQueue } from './priorityQueue'
import { wavBase64ToAudioBuffer } from './wavDecoder'
import type {
  EngineerAudioMsg,
  EngineerClientMsg,
  EngineerInstallCompleteMsg,
  EngineerInstallProgressMsg,
  EngineerServerMsg,
  EngineerStatusMsg,
  VoiceInfo,
} from '../types'

export type EngineerStatus =
  | { kind: 'unknown' }
  | { kind: 'connected'; piperInstalled: boolean; piperVersion: string | null; voices: VoiceInfo[] }
  | { kind: 'error'; message: string }

export type StatusListener = (status: EngineerStatus) => void
export type InstallProgressListener = (msg: EngineerInstallProgressMsg) => void
export type InstallCompleteListener = (msg: EngineerInstallCompleteMsg) => void
export type AudioListener = (msg: EngineerAudioMsg) => void

class AudioEngineerService {
  private ctx: AudioContext | null = null
  private gainNode: GainNode | null = null
  private noiseBuffer: AudioBuffer | null = null

  private queue = new PriorityQueue()
  private isPlaying = false
  private currentSources: AudioBufferSourceNode[] = []
  private currentNoiseSources: AudioBufferSourceNode[] = []

  private volume = 0.7
  private radioEffect: RadioEffectMode = 'subtle'
  private enabled = false
  private outputDeviceId: string | null = null

  private sendCmd: ((msg: EngineerClientMsg) => void) | null = null

  private statusListeners: Set<StatusListener> = new Set()
  private progressListeners: Set<InstallProgressListener> = new Set()
  private completeListeners: Set<InstallCompleteListener> = new Set()
  private audioListeners: Set<AudioListener> = new Set()

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  init(sendCmd: (msg: EngineerClientMsg) => void) {
    this.sendCmd = sendCmd
  }

  destroy() {
    this.queue.clear()
    this.stopCurrentPlayback()
    this.ctx?.close()
    this.ctx = null
    this.gainNode = null
  }

  // ---------------------------------------------------------------------------
  // Incoming server messages (called from App.tsx's onMessage handler)
  // ---------------------------------------------------------------------------

  handleMessage(msg: EngineerServerMsg) {
    switch (msg.type) {
      case 'engineerStatus':
        this.onStatus(msg)
        break
      case 'engineerInstallProgress':
        this.progressListeners.forEach((l) => l(msg))
        break
      case 'engineerInstallComplete':
        this.completeListeners.forEach((l) => l(msg))
        break
      case 'engineerAudio':
        if (this.enabled) this.onAudio(msg)
        this.audioListeners.forEach((l) => l(msg))
        break
      case 'engineerError':
        console.warn('[engineer]', msg.message)
        break
    }
  }

  // ---------------------------------------------------------------------------
  // Message listeners (for UI components)
  // ---------------------------------------------------------------------------

  onStatusChange(listener: StatusListener) {
    this.statusListeners.add(listener)
    return () => this.statusListeners.delete(listener)
  }

  onInstallProgress(listener: InstallProgressListener) {
    this.progressListeners.add(listener)
    return () => this.progressListeners.delete(listener)
  }

  onInstallComplete(listener: InstallCompleteListener) {
    this.completeListeners.add(listener)
    return () => this.completeListeners.delete(listener)
  }

  onAudioMsg(listener: AudioListener) {
    this.audioListeners.add(listener)
    return () => this.audioListeners.delete(listener)
  }

  // ---------------------------------------------------------------------------
  // Commands to backend
  // ---------------------------------------------------------------------------

  sendBehaviorUpdate(behavior: {
    enabled: boolean
    frequency: 'low' | 'medium' | 'high'
    muteInQualifying: boolean
    debugAllRulesInPractice: boolean
    activeVoiceId: string | null
    pilotName: string | null
    muteNameInCallouts: boolean
  }) {
    this.sendCmd?.({
      type: 'engineerUpdateBehavior',
      enabled: behavior.enabled,
      frequency: behavior.frequency,
      muteInQualifying: behavior.muteInQualifying,
      debugAllRulesInPractice: behavior.debugAllRulesInPractice,
      activeVoiceId: behavior.activeVoiceId,
      pilotName: behavior.pilotName,
      muteName: behavior.muteNameInCallouts,
    })
  }

  requestStatus() {
    this.sendCmd?.({ type: 'engineerGetStatus' })
  }

  installPiper() {
    this.sendCmd?.({ type: 'engineerInstallPiper' })
  }

  installVoice(voiceId: string) {
    this.sendCmd?.({ type: 'engineerInstallVoice', voiceId })
  }

  uninstallVoice(voiceId: string) {
    this.sendCmd?.({ type: 'engineerUninstallVoice', voiceId })
  }

  synthesize(voiceId: string, text: string, requestId: string) {
    this.sendCmd?.({ type: 'engineerSynthesize', voiceId, text, requestId })
  }

  // ---------------------------------------------------------------------------
  // Settings
  // ---------------------------------------------------------------------------

  setEnabled(enabled: boolean) {
    this.enabled = enabled
    if (!enabled) {
      this.queue.clear()
      this.stopCurrentPlayback()
    }
  }

  setVolume(volume: number) {
    this.volume = volume
    if (this.gainNode) this.gainNode.gain.value = volume
  }

  setRadioEffect(mode: RadioEffectMode) {
    this.radioEffect = mode
  }

  async setOutputDevice(deviceId: string | null) {
    this.outputDeviceId = deviceId
    await this.applySinkId()
  }

  // Apply the stored outputDeviceId to the current AudioContext (if supported).
  // AudioContext.setSinkId() is available in Chrome/Edge 110+; we cast defensively.
  private async applySinkId() {
    const ctx = this.ctx as (AudioContext & { setSinkId?: (id: string) => Promise<void> }) | null
    if (!ctx || typeof ctx.setSinkId !== 'function') return
    try {
      // Empty string restores the system default output device.
      await ctx.setSinkId(this.outputDeviceId ?? '')
    } catch (e) {
      console.warn('[engineer] setSinkId failed', e)
    }
  }

  // ---------------------------------------------------------------------------
  // Audio playback
  // ---------------------------------------------------------------------------

  private getAudioContext(): AudioContext {
    if (!this.ctx || this.ctx.state === 'closed') {
      this.ctx = new AudioContext()
      this.gainNode = this.ctx.createGain()
      this.gainNode.gain.value = this.volume
      this.gainNode.connect(this.ctx.destination)
      this.initNoise()
      // Apply stored output device to the fresh context (no-op if setSinkId unsupported).
      void this.applySinkId()
    }
    return this.ctx
  }

  private initNoise() {
    if (!this.ctx) return
    const len = this.ctx.sampleRate * 2 // 2s loop
    const buf = this.ctx.createBuffer(1, len, this.ctx.sampleRate)
    const data = buf.getChannelData(0)
    for (let i = 0; i < len; i++) data[i] = Math.random() * 2 - 1
    this.noiseBuffer = buf
  }

  private onStatus(msg: EngineerStatusMsg) {
    this.statusListeners.forEach((l) =>
      l({
        kind: 'connected',
        piperInstalled: msg.piperInstalled,
        piperVersion: msg.piperVersion,
        voices: msg.voices,
      }),
    )
  }

  private async onAudio(msg: EngineerAudioMsg) {
    const ctx = this.getAudioContext()
    if (ctx.state === 'suspended') {
      try { await ctx.resume() } catch { /* ignore */ }
    }

    let buffer: AudioBuffer
    try {
      buffer = await wavBase64ToAudioBuffer(msg.wavBase64, ctx)
    } catch (e) {
      console.warn('[engineer] Failed to decode audio', e)
      return
    }

    const queued = this.queue.enqueue({
      requestId: msg.requestId,
      priority: msg.priority,
      buffer,
      text: msg.text,
      enqueuedAt: Date.now(),
    })

    if (!queued) return

    if (msg.priority === 'critical' && this.isPlaying) {
      this.interruptAndPlay()
    } else if (!this.isPlaying) {
      this.playNext()
    }
  }

  private interruptAndPlay() {
    // Fade out current playback in ~50ms then play next
    if (this.gainNode && this.ctx) {
      const now = this.ctx.currentTime
      this.gainNode.gain.setValueAtTime(this.gainNode.gain.value, now)
      this.gainNode.gain.linearRampToValueAtTime(0, now + 0.05)
      setTimeout(() => {
        this.stopCurrentPlayback()
        if (this.gainNode && this.ctx) {
          this.gainNode.gain.setValueAtTime(this.volume, this.ctx.currentTime)
        }
        this.playNext()
      }, 60)
    } else {
      this.stopCurrentPlayback()
      this.playNext()
    }
  }

  private stopCurrentPlayback() {
    for (const src of this.currentSources) {
      try { src.stop() } catch { /* already stopped */ }
    }
    for (const src of this.currentNoiseSources) {
      try { src.stop() } catch { /* already stopped */ }
    }
    this.currentSources = []
    this.currentNoiseSources = []
    this.isPlaying = false
  }

  private playNext() {
    const item = this.queue.dequeue()
    if (!item) {
      this.isPlaying = false
      return
    }

    const ctx = this.getAudioContext()
    const gainNode = this.gainNode!

    const source = ctx.createBufferSource()
    source.buffer = item.buffer

    const { noiseSources, pttBuffers } = connectWithEffect(
      ctx,
      source,
      gainNode,
      this.radioEffect,
      this.noiseBuffer,
    )

    this.currentSources = [source]
    this.currentNoiseSources = noiseSources
    this.isPlaying = true

    source.onended = () => {
      for (const s of noiseSources) {
        try { s.stop() } catch { /* ignore */ }
      }
      this.currentNoiseSources = []
      this.isPlaying = false
      // Small gap between clips
      setTimeout(() => this.playNext(), 300)
    }

    // PTT click before (strong mode)
    if (pttBuffers[0]) {
      const clickBefore = ctx.createBufferSource()
      clickBefore.buffer = pttBuffers[0]
      clickBefore.connect(gainNode)
      clickBefore.start()
      this.currentSources.push(clickBefore)
    }

    source.start()

    // PTT click after (strong mode)
    if (pttBuffers[1]) {
      const clickAfter = ctx.createBufferSource()
      clickAfter.buffer = pttBuffers[1]
      clickAfter.connect(gainNode)
      clickAfter.start(ctx.currentTime + item.buffer.duration + 0.05)
      this.currentSources.push(clickAfter)
    }
  }
}

export const engineerService = new AudioEngineerService()
