export interface QueuedMessage {
  requestId: string
  priority: 'critical' | 'high' | 'info'
  buffer: AudioBuffer
  text: string
  enqueuedAt: number
}

const MAX_INFO_ITEMS = 10
const MAX_AUDIO_DURATION_MS = 15_000

export class PriorityQueue {
  private items: QueuedMessage[] = []

  enqueue(item: QueuedMessage): boolean {
    const durationMs = (item.buffer.duration * 1000)
    if (durationMs > MAX_AUDIO_DURATION_MS) {
      console.warn(`[engineer] Rejected clip (${Math.round(durationMs)}ms > ${MAX_AUDIO_DURATION_MS}ms)`)
      return false
    }

    if (item.priority === 'critical') {
      // Critical goes to the very front
      this.items.unshift(item)
    } else if (item.priority === 'high') {
      // Insert after last critical
      const lastCriticalIdx = this.items.reduce(
        (acc, m, i) => (m.priority === 'critical' ? i : acc),
        -1,
      )
      this.items.splice(lastCriticalIdx + 1, 0, item)
    } else {
      // Info: FIFO, capped at MAX_INFO_ITEMS
      const infoCount = this.items.filter((m) => m.priority === 'info').length
      if (infoCount >= MAX_INFO_ITEMS) {
        // Remove oldest info item
        const oldestInfoIdx = this.items.findIndex((m) => m.priority === 'info')
        if (oldestInfoIdx !== -1) this.items.splice(oldestInfoIdx, 1)
      }
      this.items.push(item)
    }
    return true
  }

  dequeue(): QueuedMessage | undefined {
    return this.items.shift()
  }

  get length(): number {
    return this.items.length
  }

  clear(): void {
    this.items = []
  }
}
