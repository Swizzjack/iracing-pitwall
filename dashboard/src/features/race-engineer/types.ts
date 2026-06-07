/** Voice info as returned by the bridge in EngineerStatus messages. */
export interface VoiceInfo {
  id: string
  name: string
  language: string
  description: string
  sizeMb: number
  installed: boolean
  sampleUrl: string
}

/** Engineer-related ServerMessage shapes (discriminated by `type`). */
export interface EngineerStatusMsg {
  type: 'engineerStatus'
  piperInstalled: boolean
  piperVersion: string | null
  voices: VoiceInfo[]
}

export interface EngineerInstallProgressMsg {
  type: 'engineerInstallProgress'
  target: string
  targetId: string | null
  bytesDownloaded: number
  bytesTotal: number | null
  stage: string
}

export interface EngineerInstallCompleteMsg {
  type: 'engineerInstallComplete'
  target: string
  targetId: string | null
  success: boolean
  error: string | null
}

export interface EngineerAudioMsg {
  type: 'engineerAudio'
  requestId: string
  priority: 'critical' | 'high' | 'info'
  wavBase64: string
  sampleRate: number
  durationMs: number
  text: string
}

export interface EngineerErrorMsg {
  type: 'engineerError'
  message: string
}

export type EngineerServerMsg =
  | EngineerStatusMsg
  | EngineerInstallProgressMsg
  | EngineerInstallCompleteMsg
  | EngineerAudioMsg
  | EngineerErrorMsg

export type EngineerClientMsg =
  | { type: 'engineerGetStatus' }
  | { type: 'engineerInstallPiper' }
  | { type: 'engineerInstallVoice'; voiceId: string }
  | { type: 'engineerUninstallVoice'; voiceId: string }
  | { type: 'engineerSynthesize'; voiceId: string; text: string; requestId: string }
  | {
      type: 'engineerUpdateBehavior'
      enabled: boolean
      frequency: 'low' | 'medium' | 'high'
      muteInQualifying: boolean
      debugAllRulesInPractice: boolean
      activeVoiceId: string | null
      pilotName: string | null
      muteName: boolean
    }
