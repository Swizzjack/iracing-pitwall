import { useEffect, useRef, useState } from 'react'
import type { LayoutItem } from 'react-grid-layout'
import type { ServerMessage } from '@shared/ServerMessage'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import type { StandingsSnapshot } from '@shared/StandingsSnapshot'
import type { SessionInfoYaml } from '@shared/SessionInfoYaml'
import type { TrackMapSnapshot } from '@shared/TrackMapSnapshot'
import { WsClient, type ConnectionState } from './ws/Client'
import { Dashboard } from './layout/Dashboard'
import { EditToolbar } from './layout/EditToolbar'
import { loadLayout, saveLayout, resetLayout, type StoredLayout } from './layout/storage'
import { SettingsDrawer } from './components/SettingsDrawer'
import './App.css'

const UI_SCALE_KEY = 'iracing-ui-scale-v1'

function loadUiScale(): number {
  const raw = localStorage.getItem(UI_SCALE_KEY)
  const n = raw ? parseFloat(raw) : NaN
  return Number.isFinite(n) && n >= 0.8 && n <= 2.0 ? n : 1.0
}

const WS_URL = import.meta.env.DEV
  ? 'ws://127.0.0.1:8765/ws'
  : `ws://${location.host}/ws`

function App() {
  const [conn, setConn] = useState<ConnectionState>('closed')
  const [bridgeVersion, setBridgeVersion] = useState<string | null>(null)
  const [lanUrl, setLanUrl] = useState<string | null>(null)
  const [tel, setTel] = useState<TelemetrySnapshot | null>(null)
  const [standings, setStandings] = useState<StandingsSnapshot | null>(null)
  const [info, setInfo] = useState<SessionInfoYaml | null>(null)
  const [trackMap, setTrackMap] = useState<TrackMapSnapshot | null>(null)
  const [isFs, setIsFs] = useState(false)
  const [editing, setEditing] = useState(false)
  const [stored, setStored] = useState<StoredLayout>(loadLayout)
  const [uiScale, setUiScale] = useState<number>(loadUiScale)
  const [showGlobalSettings, setShowGlobalSettings] = useState(false)

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

  const toggleFs = () => {
    if (document.fullscreenElement) document.exitFullscreen()
    else document.documentElement.requestFullscreen()
  }

  useEffect(() => {
    const client = new WsClient(WS_URL)
    clientRef.current = client
    const offState = client.onState(setConn)
    const offMsg = client.onMessage((msg: ServerMessage) => {
      switch (msg.type) {
        case 'hello':
          setBridgeVersion(msg.bridgeVersion)
          setLanUrl(msg.lanUrl ?? null)
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
        case 'sdkStatus':
        case 'disconnected':
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

  return (
    <div className="app">
      <header>
        <h1>iRacing Pitwall</h1>
        <div className="header-right">
          <div className={`status status-${conn}`}>
            <span className="dot" />
            {conn}
            {bridgeVersion && <span className="ver">v{bridgeVersion}</span>}
          </div>
          {lanUrl && (
            <a className="lan-url" href={lanUrl} target="_blank" rel="noreferrer" title="LAN-Adresse — auf anderen Geräten öffnen">
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
          <button
            className={`fs-btn${showGlobalSettings ? ' header-btn-active' : ''}`}
            onClick={() => setShowGlobalSettings(v => !v)}
            title="UI Settings"
          >⚙</button>
        </div>
      </header>
      <SettingsDrawer
        open={showGlobalSettings}
        onClose={() => setShowGlobalSettings(false)}
        title="UI Settings"
        variant="global"
      >
        <div className="settings-drawer-footer">
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
          data={{ tel, standings, info, trackMap }}
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
