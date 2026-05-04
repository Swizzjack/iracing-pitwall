import { useEffect, useRef, useState } from 'react'
import type { ServerMessage } from '@shared/ServerMessage'
import type { TelemetrySnapshot } from '@shared/TelemetrySnapshot'
import type { StandingsSnapshot } from '@shared/StandingsSnapshot'
import type { SessionInfoYaml } from '@shared/SessionInfoYaml'
import { WsClient, type ConnectionState } from './ws/Client'
import { Telemetry } from './widgets/Telemetry'
import { Standings } from './widgets/Standings'
import { SessionInfo } from './widgets/SessionInfo'
import './App.css'

const WS_URL = import.meta.env.DEV
  ? 'ws://127.0.0.1:8765/ws'
  : `ws://${location.host}/ws`

function App() {
  const [conn, setConn] = useState<ConnectionState>('closed')
  const [bridgeVersion, setBridgeVersion] = useState<string | null>(null)
  const [tel, setTel] = useState<TelemetrySnapshot | null>(null)
  const [standings, setStandings] = useState<StandingsSnapshot | null>(null)
  const [info, setInfo] = useState<SessionInfoYaml | null>(null)

  // 60 Hz telemetry → throttle to one render per animation frame.
  const pendingTel = useRef<TelemetrySnapshot | null>(null)
  const rafScheduled = useRef(false)

  useEffect(() => {
    const client = new WsClient(WS_URL)
    const offState = client.onState(setConn)
    const offMsg = client.onMessage((msg: ServerMessage) => {
      switch (msg.type) {
        case 'hello':
          setBridgeVersion(msg.bridgeVersion)
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
    }
  }, [])

  return (
    <div className="app">
      <header>
        <h1>iRacing Pitwall</h1>
        <div className={`status status-${conn}`}>
          <span className="dot" />
          {conn}
          {bridgeVersion && <span className="ver">v{bridgeVersion}</span>}
        </div>
      </header>
      <main>
        <Telemetry snap={tel} />
        <SessionInfo info={info} />
        <Standings snap={standings} playerCarIdx={tel?.playerCarIdx ?? null} />
      </main>
    </div>
  )
}

export default App
