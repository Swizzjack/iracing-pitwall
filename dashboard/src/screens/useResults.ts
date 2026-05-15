import { useCallback, useEffect, useRef, useState } from 'react'
import type { WsClient } from '../ws/Client'
import type { SessionSummary } from '@shared/SessionSummary'
import type { SessionDetail } from '@shared/SessionDetail'
import type { FilterOptions } from '@shared/FilterOptions'
import type { ResultsFilter } from '@shared/ResultsFilter'
import type { LapRow } from '@shared/LapRow'

export interface ResultsState {
  sessions: SessionSummary[]
  total: number
  detail: SessionDetail | null
  filterOptions: FilterOptions | null
  loading: boolean
  oauthLinked: boolean
  oauthMemberName: string | null
  laps: LapRow[] | null        // per-driver laps (LapByLapView)
  lapsCarIdx: number | null
  chartLaps: LapRow[] | null   // all-cars laps (Charts tab)
}

const DEFAULT_FILTER: ResultsFilter = {
  trackId: null,
  carId: null,
  seriesId: null,
  eventType: null,
  dateFrom: null,
  dateTo: null,
  limit: 50,
  offset: null,
}

export function useResults(client: WsClient, active: boolean) {
  const chartLoadPending = useRef(false)

  const [state, setState] = useState<ResultsState>({
    sessions: [],
    total: 0,
    detail: null,
    filterOptions: null,
    loading: false,
    oauthLinked: false,
    oauthMemberName: null,
    laps: null,
    lapsCarIdx: null,
    chartLaps: null,
  })
  const [filter, setFilter] = useState<ResultsFilter>(DEFAULT_FILTER)
  const filterRef = useRef(filter)
  filterRef.current = filter

  const query = useCallback(
    (f: ResultsFilter) => {
      setState((s) => ({ ...s, loading: true }))
      client.send({ type: 'queryResults', filter: f })
    },
    [client],
  )

  const loadDetail = useCallback(
    (subSessionId: number) => {
      client.send({ type: 'queryResultDetail', subSessionId })
    },
    [client],
  )

  const closeDetail = useCallback(() => {
    setState((s) => ({ ...s, detail: null, laps: null, lapsCarIdx: null, chartLaps: null }))
  }, [])

  const loadChartLaps = useCallback(
    (subSessionId: number) => {
      chartLoadPending.current = true
      setState((s) => ({ ...s, chartLaps: null }))
      client.send({ type: 'queryLaps', subSessionId, carIdx: null })
    },
    [client],
  )

  const loadLaps = useCallback(
    (subSessionId: number, carIdx: number | null) => {
      setState((s) => ({ ...s, laps: null, lapsCarIdx: carIdx }))
      client.send({ type: 'queryLaps', subSessionId, carIdx })
    },
    [client],
  )

  const closeLaps = useCallback(() => {
    setState((s) => ({ ...s, laps: null, lapsCarIdx: null }))
  }, [])

  const startOAuth = useCallback(() => {
    client.send({ type: 'startOAuth' })
  }, [client])

  const triggerFetch = useCallback(
    (subSessionId: number) => {
      client.send({ type: 'triggerFetch', subSessionId })
    },
    [client],
  )

  useEffect(() => {
    const off = client.onMessage((msg) => {
      switch (msg.type) {
        case 'oAuthStatus':
          setState((s) => ({
            ...s,
            oauthLinked: msg.linked,
            oauthMemberName: msg.memberName ?? null,
          }))
          break
        case 'oAuthUrl':
          window.open(msg.url, '_blank', 'noopener,noreferrer')
          break
        case 'resultsList':
          setState((s) => ({
            ...s,
            sessions: msg.sessions,
            total: msg.total,
            loading: false,
          }))
          break
        case 'resultDetail':
          setState((s) => ({ ...s, detail: msg.session }))
          break
        case 'filterOptions':
          setState((s) => ({ ...s, filterOptions: msg.options }))
          break
        case 'resultsUpdated':
          query(filterRef.current)
          break
        case 'lapsList':
          if (chartLoadPending.current) {
            chartLoadPending.current = false
            setState((s) => ({ ...s, chartLaps: msg.laps }))
          } else {
            setState((s) => ({ ...s, laps: msg.laps }))
          }
          break
      }
    })
    return off
  }, [client, query])

  useEffect(() => {
    if (!active) return
    client.send({ type: 'queryFilterOptions' })
    query(filter)
  }, [active]) // eslint-disable-line react-hooks/exhaustive-deps

  const applyFilter = useCallback(
    (patch: Partial<ResultsFilter>) => {
      const next = { ...filterRef.current, ...patch, offset: null }
      setFilter(next)
      query(next)
    },
    [query],
  )

  return { state, filter, applyFilter, loadDetail, closeDetail, loadLaps, closeLaps, loadChartLaps, startOAuth, triggerFetch }
}
