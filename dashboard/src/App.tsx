import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { LayoutItem } from 'react-grid-layout'
import type { ServerMessage } from '@shared/ServerMessage'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import type { StandingsSnapshot } from '@shared/StandingsSnapshot'
import type { SessionInfoYaml } from '@shared/SessionInfoYaml'
import type { TrackMapSnapshot } from '@shared/TrackMapSnapshot'
import type { SdkDebugSnapshot } from '@shared/SdkDebugSnapshot'
import { SdkDebugPanel } from './features/sdk-debug/SdkDebugPanel'
import { WsClient, type ConnectionState } from './ws/Client'
import { Dashboard } from './layout/Dashboard'
import { EditToolbar } from './layout/EditToolbar'
import { loadLayout, saveLayout, resetLayout, type StoredLayout } from './layout/storage'
import { SettingsDrawer } from './components/SettingsDrawer'
import { RaceEngineerPage } from './features/race-engineer/RaceEngineerPage'
import { engineerService } from './features/race-engineer/audio/AudioEngineerService'
import {
  loadEngineerSettings,
  saveEngineerSettings,
  type EngineerSettings,
} from './features/race-engineer/state/engineerSettings'
import type { EngineerServerMsg } from './features/race-engineer/types'
import './App.css'

const UI_SCALE_KEY = 'iracing-ui-scale-v1'
const DEV_MODE_KEY = 'iracing-dev-mode-v1'

function loadDevMode(): boolean {
  return localStorage.getItem(DEV_MODE_KEY) === '1'
}

function loadUiScale(): number {
  const raw = localStorage.getItem(UI_SCALE_KEY)
  const n = raw ? parseFloat(raw) : NaN
  return Number.isFinite(n) && n >= 0.8 && n <= 2.0 ? n : 1.0
}

const WS_URL = import.meta.env.DEV
  ? 'ws://127.0.0.1:8765/ws'
  : `ws://${location.host}/ws`

/** Type-guard: is this ServerMessage one of the engineer subtypes? */
function isEngineerMsg(msg: ServerMessage): boolean {
  const t = msg.type as string
  return (
    t === 'engineerStatus' ||
    t === 'engineerInstallProgress' ||
    t === 'engineerInstallComplete' ||
    t === 'engineerAudio' ||
    t === 'engineerError'
  )
}

function App() {
  const [conn, setConn] = useState<ConnectionState>('closed')
  const [bridgeVersion, setBridgeVersion] = useState<string | null>(null)
  const [lanUrl, setLanUrl] = useState<string | null>(null)
  const [availableUpdate, setAvailableUpdate] = useState<{ latestVersion: string; releaseUrl: string } | null>(null)
  const [tel, setTel] = useState<TelemetrySnapshot | null>(null)
  const [standings, setStandings] = useState<StandingsSnapshot | null>(null)
  const [info, setInfo] = useState<SessionInfoYaml | null>(null)
  const [trackMap, setTrackMap] = useState<TrackMapSnapshot | null>(null)
  const [isFs, setIsFs] = useState(false)
  const [editing, setEditing] = useState(false)
  const [stored, setStored] = useState<StoredLayout>(loadLayout)
  const [uiScale, setUiScale] = useState<number>(loadUiScale)
  const [showGlobalSettings, setShowGlobalSettings] = useState(false)
  const [showEngineer, setShowEngineer] = useState(false)

  // Hidden admin/debug view — unlocked via Ctrl+Shift+D, persisted locally.
  // Not advertised anywhere in the UI; the header button only appears once unlocked.
  const [devMode, setDevMode] = useState<boolean>(loadDevMode)
  const [showSdkDebug, setShowSdkDebug] = useState(false)
  const [sdkDebug, setSdkDebug] = useState<SdkDebugSnapshot | null>(null)

  // Race Engineer settings
  const [engineerSettings, setEngineerSettings] = useState<EngineerSettings>(loadEngineerSettings)
  const engineerSettingsRef = useRef<EngineerSettings>(engineerSettings)

  // 60 Hz telemetry → throttle to one render per animation frame
  const pendingTel = useRef<TelemetrySnapshot | null>(null)
  const rafScheduled = useRef(false)
  const saveTimeout = useRef<ReturnType<typeof setTimeout> | null>(null)
  const clientRef = useRef<WsClient | null>(null)

  useEffect(() => {
    const onChange = () => setIsFs(!!document.fullscreenElement)
    document.addEventListener('fullscreenchange', onChange)
    return () => document.removeEventListener('fullscreenchange', onChange)
  }, [])

  useEffect(() => {
    document.documentElement.style.setProperty('--ui-scale', String(uiScale))
    localStorage.setItem(UI_SCALE_KEY, String(uiScale))
  }, [uiScale])

  // Hidden unlock: Ctrl+Shift+D toggles the admin/debug mode and persists it.
  // Deliberately undocumented — not surfaced anywhere in the visible UI.
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.ctrlKey && e.shiftKey && (e.key === 'D' || e.key === 'd')) {
        e.preventDefault()
        setDevMode((prev) => {
          const next = !prev
          localStorage.setItem(DEV_MODE_KEY, next ? '1' : '0')
          if (!next) setShowSdkDebug(false)
          return next
        })
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [])

  // Initialize engineer service with a command sender
  useEffect(() => {
    engineerService.init((msg) => {
      clientRef.current?.send(msg as never)
    })
    // Sync initial settings to backend on mount
    engineerService.setEnabled(engineerSettings.enabled)
    engineerService.setVolume(engineerSettings.volume)
    engineerService.setRadioEffect(engineerSettings.radioEffect)
    void engineerService.setOutputDevice(engineerSettings.outputDeviceId)

    return () => engineerService.destroy()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const toggleFs = () => {
    if (document.fullscreenElement) document.exitFullscreen()
    else document.documentElement.requestFullscreen()
  }

  useEffect(() => {
    const client = new WsClient(WS_URL)
    clientRef.current = client
    const offState = client.onState(setConn)
    const offMsg = client.onMessage((msg: ServerMessage) => {
      // Forward engineer messages to the audio service
      if (isEngineerMsg(msg)) {
        engineerService.handleMessage(msg as unknown as EngineerServerMsg)
        return
      }

      switch (msg.type) {
        case 'hello':
          setBridgeVersion(msg.bridgeVersion)
          setLanUrl(msg.lanUrl ?? null)
          // Re-sync engineer behavior on every fresh bridge connection
          engineerService.sendBehaviorUpdate({
            enabled: engineerSettingsRef.current.enabled,
            frequency: engineerSettingsRef.current.frequency,
            muteInQualifying: engineerSettingsRef.current.muteInQualifying,
            debugAllRulesInPractice: engineerSettingsRef.current.debugAllRulesInPractice,
            activeVoiceId: engineerSettingsRef.current.activeVoiceId,
            pilotName: engineerSettingsRef.current.pilotName || null,
            muteNameInCallouts: engineerSettingsRef.current.muteNameInCallouts,
          })
          break
        case 'telemetry':
          pendingTel.current = msg.snapshot
          if (!rafScheduled.current) {
            rafScheduled.current = true
            requestAnimationFrame(() => {
              rafScheduled.current = false
              if (pendingTel.current) setTel(pendingTel.current)
            })
          }
          break
        case 'standings':
          setStandings(msg.snapshot)
          break
        case 'sessionInfo':
          setInfo(msg.info)
          break
        case 'trackMap':
          setTrackMap(msg.snapshot)
          break
        case 'updateAvailable':
          setAvailableUpdate({ latestVersion: msg.latestVersion, releaseUrl: msg.releaseUrl })
          break
        case 'sdkDebug':
          setSdkDebug(msg.snapshot)
          break
      }
    })
    client.connect()
    return () => {
      offState()
      offMsg()
      client.stop()
      clientRef.current = null
    }
  }, [])

  function handleLayoutChange(newLayout: readonly LayoutItem[]) {
    setStored((prev) => {
      const next = { ...prev, layout: [...newLayout] }
      if (saveTimeout.current) clearTimeout(saveTimeout.current)
      saveTimeout.current = setTimeout(() => saveLayout(next), 200)
      return next
    })
  }

  function handleRemove(id: string) {
    setStored((prev) => {
      const next = { ...prev, visible: prev.visible.filter((v) => v !== id) }
      saveLayout(next)
      return next
    })
  }

  function handleAdd(id: string) {
    setStored((prev) => {
      if (prev.visible.includes(id)) return prev
      const next = { ...prev, visible: [...prev.visible, id] }
      saveLayout(next)
      return next
    })
  }

  function handleReset() {
    const defaults = resetLayout()
    setStored(defaults)
  }

  const handleDeleteTrackMap = useCallback((trackKey: string) => {
    clientRef.current?.send({ type: 'deleteTrackMap', trackKey } as never)
  }, [])

  // Stable object identity so memoized widgets only re-render when the
  // snapshot they consume actually changed (standings at 4 Hz, not 60 Hz).
  const widgetData = useMemo(
    () => ({ tel, standings, info, trackMap, onDeleteTrackMap: handleDeleteTrackMap }),
    [tel, standings, info, trackMap, handleDeleteTrackMap],
  )

  function handleEngineerSettingsChange(partial: Partial<EngineerSettings>) {
    // Compute outside the setState updater: updaters must stay pure
    // (StrictMode runs them twice, which would double every side effect).
    // engineerSettingsRef is the always-current source of truth.
    const next = { ...engineerSettingsRef.current, ...partial }
    engineerSettingsRef.current = next
    saveEngineerSettings(next)
    // Sync changed settings to the service / backend
    if ('enabled' in partial) engineerService.setEnabled(next.enabled)
    if ('volume' in partial) engineerService.setVolume(next.volume)
    if ('radioEffect' in partial) engineerService.setRadioEffect(next.radioEffect)
    if ('outputDeviceId' in partial) void engineerService.setOutputDevice(next.outputDeviceId)
    // Send behaviour update for backend-relevant settings
    engineerService.sendBehaviorUpdate({
      enabled: next.enabled,
      frequency: next.frequency,
      muteInQualifying: next.muteInQualifying,
      debugAllRulesInPractice: next.debugAllRulesInPractice,
      activeVoiceId: next.activeVoiceId,
      pilotName: next.pilotName || null,
      muteNameInCallouts: next.muteNameInCallouts,
    })
    setEngineerSettings(next)
  }

  return (
    <div className="app">
      <header>
        <h1>iRacing Pitwall</h1>
        <div className="header-right">
          <div className={`status status-${conn}`}>
            <span className="dot" />
            {conn}
            {bridgeVersion && <span className="ver">v{bridgeVersion}</span>}
            {availableUpdate && (
              <a
                className="update-pill"
                href={availableUpdate.releaseUrl}
                target="_blank"
                rel="noreferrer"
                title={`v${availableUpdate.latestVersion} available — click to open release page`}
              >
                ⬆ update
              </a>
            )}
          </div>
          {lanUrl && (
            <a className="lan-url" href={lanUrl} target="_blank" rel="noreferrer" title="LAN address — open on other devices">
              {lanUrl.replace('http://', '').replace(/\/$/, '')}
            </a>
          )}
          <EditToolbar
            editing={editing}
            visible={stored.visible}
            onToggleEdit={() => setEditing((e) => !e)}
            onAdd={handleAdd}
            onReset={handleReset}
          />
          <button className="fs-btn" onClick={toggleFs} title={isFs ? 'Exit fullscreen' : 'Enter fullscreen'}>
            {isFs ? '⤡' : '⤢'}
          </button>
          {/* Race Engineer button */}
          <button
            className={`fs-btn${showEngineer ? ' header-btn-active' : ''}`}
            onClick={() => { setShowEngineer(v => !v); setShowGlobalSettings(false) }}
            title="Race Engineer"
          >🎙</button>
          {/* Hidden admin/debug view — only present once unlocked via Ctrl+Shift+D */}
          {devMode && (
            <button
              className={`fs-btn${showSdkDebug ? ' header-btn-active' : ''}`}
              onClick={() => setShowSdkDebug(v => !v)}
              title="SDK Admin / Debug View"
            >🐞</button>
          )}
          <button
            className={`fs-btn${showGlobalSettings ? ' header-btn-active' : ''}`}
            onClick={() => { setShowGlobalSettings(v => !v); setShowEngineer(false) }}
            title="UI Settings"
          >⚙</button>
        </div>
      </header>

      {/* Race Engineer panel */}
      {showEngineer && (
        <div
          style={{
            position: 'fixed',
            top: 48,
            right: 0,
            width: 340,
            maxHeight: 'calc(100vh - 56px)',
            overflowY: 'auto',
            background: '#111',
            borderLeft: '1px solid #222',
            zIndex: 200,
            padding: '16px 16px 24px',
          }}
        >
          <RaceEngineerPage
            settings={engineerSettings}
            onSettingsChange={handleEngineerSettingsChange}
            onClose={() => setShowEngineer(false)}
          />
        </div>
      )}

      {/* Hidden admin/debug view — full-screen overlay, only reachable via the unlocked header button */}
      {devMode && showSdkDebug && (
        <SdkDebugPanel
          snapshot={sdkDebug}
          send={(msg) => clientRef.current?.send(msg)}
          onClose={() => setShowSdkDebug(false)}
        />
      )}

      <SettingsDrawer
        open={showGlobalSettings}
        onClose={() => setShowGlobalSettings(false)}
        title="UI Settings"
        variant="global"
      >
        <div className="settings-drawer-footer">
          <div className="settings-section">
            <div className="settings-section-title">Updates</div>
            {availableUpdate ? (
              <div>
                <div style={{ color: '#facc15', fontSize: 'calc(13px * var(--ui-scale, 1))', marginBottom: 8 }}>
                  v{availableUpdate.latestVersion} is available
                </div>
                <button
                  className="header-btn"
                  style={{ width: '100%', color: '#facc15', borderColor: '#facc15' }}
                  onClick={() => window.open(availableUpdate.releaseUrl, '_blank')}
                >
                  ⬇ Download v{availableUpdate.latestVersion}
                </button>
                <div style={{ color: '#555', fontSize: 'calc(11px * var(--ui-scale, 1))', marginTop: 8, lineHeight: 1.4 }}>
                  Download the new .exe, close this app, replace the old file and restart.
                </div>
              </div>
            ) : (
              <div style={{ color: '#555', fontSize: 'calc(12px * var(--ui-scale, 1))' }}>
                You're on the latest version.
              </div>
            )}
          </div>
          <div className="settings-section">
            <div className="settings-section-title">UI Scale</div>
            <div className="settings-footer-row">
              <label style={{ color: '#888', fontSize: 'var(--settings-fs)' }}>Size</label>
              <input
                type="range" min={0.8} max={2.0} step={0.05}
                value={uiScale}
                onChange={e => setUiScale(parseFloat(e.target.value))}
                style={{ flex: 1 }}
              />
              <span style={{ color: '#888', fontSize: 'var(--settings-fs)', minWidth: 36, textAlign: 'right' }}>
                {Math.round(uiScale * 100)}%
              </span>
            </div>
            <div style={{ color: '#555', fontSize: 'calc(11px * var(--ui-scale, 1))', marginTop: 8, lineHeight: 1.4 }}>
              Scales header, buttons and widget contents globally.
              Multiplied with per-widget scale sliders.
            </div>
            {uiScale !== 1.0 && (
              <button
                className="header-btn header-btn-danger"
                style={{ marginTop: 10, width: '100%' }}
                onClick={() => setUiScale(1.0)}
              >Reset (100%)</button>
            )}
          </div>
        </div>
      </SettingsDrawer>
      <main>
        <Dashboard
          data={widgetData}
          visible={stored.visible}
          layout={stored.layout}
          editing={editing}
          onLayoutChange={handleLayoutChange}
          onRemove={handleRemove}
        />
      </main>
    </div>
  )
}

export default App
