import type { ServerMessage } from '@shared/ServerMessage'
import type { ClientMessage } from '@shared/ClientMessage'

export type ConnectionState = 'connecting' | 'open' | 'closed'

type MessageListener = (msg: ServerMessage) => void
type StateListener = (state: ConnectionState) => void

export class WsClient {
  private url: string
  private ws: WebSocket | null = null
  private messageListeners = new Set<MessageListener>()
  private stateListeners = new Set<StateListener>()
  private retry = 0
  private state: ConnectionState = 'closed'
  private reconnectTimer: number | null = null
  private stopped = false

  constructor(url: string) {
    this.url = url
  }

  connect(): void {
    if (this.stopped) return
    this.setState('connecting')
    const ws = new WebSocket(this.url)
    this.ws = ws

    ws.onopen = () => {
      this.retry = 0
      this.setState('open')
    }
    ws.onmessage = (ev) => {
      try {
        const msg = JSON.parse(ev.data as string) as ServerMessage
        for (const l of this.messageListeners) l(msg)
      } catch (e) {
        console.error('ws parse error', e, ev.data)
      }
    }
    ws.onclose = () => {
      this.setState('closed')
      this.scheduleReconnect()
    }
    ws.onerror = () => {
      // onclose will follow and trigger reconnect
    }
  }

  send(msg: ClientMessage): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg))
    }
  }

  stop(): void {
    this.stopped = true
    if (this.reconnectTimer != null) {
      window.clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    this.ws?.close()
  }

  onMessage(l: MessageListener): () => void {
    this.messageListeners.add(l)
    return () => { this.messageListeners.delete(l) }
  }

  onState(l: StateListener): () => void {
    this.stateListeners.add(l)
    l(this.state)
    return () => { this.stateListeners.delete(l) }
  }

  private scheduleReconnect(): void {
    if (this.stopped || this.reconnectTimer != null) return
    const delay = Math.min(500 * 2 ** this.retry, 8_000)
    this.retry++
    this.reconnectTimer = window.setTimeout(() => {
      this.reconnectTimer = null
      this.connect()
    }, delay)
  }

  private setState(s: ConnectionState): void {
    if (this.state === s) return
    this.state = s
    for (const l of this.stateListeners) l(s)
  }
}
