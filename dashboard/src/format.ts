export function fmtLapTime(seconds: number | null | undefined): string {
  if (!(seconds != null && seconds > 0)) return '—'
  const m = Math.floor(seconds / 60)
  const s = seconds - m * 60
  return `${m}:${s.toFixed(3).padStart(6, '0')}`
}

export function fmtLapMs(ms: number | null | undefined): string {
  if (ms == null || ms <= 0) return '—'
  return fmtLapTime(ms / 1000)
}

export function parseSectors(json: string | null | undefined): number[] {
  if (!json) return []
  try { return JSON.parse(json) } catch { return [] }
}

export function hexFromClassColor(n: number | bigint | null | undefined): string | null {
  if (n == null) return null
  const v = typeof n === 'bigint' ? Number(n) : n
  return `#${Math.max(0, Math.min(0xffffff, v)).toString(16).padStart(6, '0')}`
}

export function readableTextOn(bg: string): string {
  const r = parseInt(bg.slice(1, 3), 16)
  const g = parseInt(bg.slice(3, 5), 16)
  const b = parseInt(bg.slice(5, 7), 16)
  return (0.299 * r + 0.587 * g + 0.114 * b) / 255 > 0.55 ? '#000' : '#fff'
}

export const LIC_COLORS: Record<string, { bg: string; fg: string }> = {
  R: { bg: 'rgba(60, 0, 0, 0.82)',    fg: '#ff7575' },
  D: { bg: 'rgba(60, 22, 0, 0.82)',   fg: '#ffaa55' },
  C: { bg: 'rgba(55, 44, 0, 0.82)',   fg: '#ffe060' },
  B: { bg: 'rgba(0, 45, 18, 0.82)',   fg: '#55dd8a' },
  A: { bg: 'rgba(0, 30, 60, 0.82)',   fg: '#66b2ff' },
  P: { bg: 'rgba(38, 0, 62, 0.82)',   fg: '#cc7fff' },
  W: { bg: 'rgba(40, 34, 0, 0.82)',   fg: '#ffd966' },
}

/**
 * iRacing Strength-of-Field over a set of iRatings:
 * SoF = (1600/ln 2) · ln( n / Σ 2^(−r/1600) ). Null for an empty set.
 */
export function sofFromRatings(ratings: number[]): number | null {
  if (ratings.length === 0) return null
  const BR = 1600 / Math.LN2
  return BR * Math.log(ratings.length / ratings.reduce((s, r) => s + Math.pow(2, -r / 1600), 0))
}

export function fmtDuration(sec: number): string {
  if (!(sec >= 0)) return '—'
  const h = Math.floor(sec / 3600)
  const m = Math.floor((sec % 3600) / 60)
  const s = Math.floor(sec % 60)
  if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
  return `${m}:${String(s).padStart(2, '0')}`
}
