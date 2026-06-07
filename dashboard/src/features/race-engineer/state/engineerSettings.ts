//! Race Engineer settings — persisted in localStorage.
//!
//! Follows the project's load/save pattern (no Zustand).
//! Key: 'iracing-race-engineer-v1'

import type { RadioEffectMode } from '../audio/radioEffect'

export interface EngineerSettings {
  enabled: boolean
  activeVoiceId: string | null
  volume: number
  frequency: 'low' | 'medium' | 'high'
  radioEffect: RadioEffectMode
  muteInQualifying: boolean
  debugAllRulesInPractice: boolean
  pilotName: string
  muteNameInCallouts: boolean
  /** Audio output device id (from mediaDevices.enumerateDevices). null = system default. */
  outputDeviceId: string | null
}

const SETTINGS_KEY = 'iracing-race-engineer-v1'
const SETUP_KEY = 'iracing-race-engineer-setup-v1'

const DEFAULTS: EngineerSettings = {
  enabled: false,
  activeVoiceId: null,
  volume: 1.0,
  frequency: 'medium',
  radioEffect: 'subtle',
  muteInQualifying: false,
  debugAllRulesInPractice: false,
  pilotName: '',
  muteNameInCallouts: false,
  outputDeviceId: null,
}

export function loadEngineerSettings(): EngineerSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY)
    if (!raw) return { ...DEFAULTS }
    const parsed = JSON.parse(raw) as Partial<EngineerSettings>
    return { ...DEFAULTS, ...parsed }
  } catch {
    return { ...DEFAULTS }
  }
}

export function saveEngineerSettings(settings: EngineerSettings): void {
  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings))
  } catch { /* ignore */ }
}

export function isSetupCompleted(): boolean {
  return localStorage.getItem(SETUP_KEY) === 'true'
}

export function markSetupCompleted(): void {
  localStorage.setItem(SETUP_KEY, 'true')
}

export function resetSetup(): void {
  localStorage.removeItem(SETUP_KEY)
}
