import { useCallback, useEffect, useRef, useState } from 'react'
import type { WsClient } from '../ws/Client'
import type { SessionSummary } from '@shared/SessionSummary'
import type { SessionDetail } from '@shared/SessionDetail'
import type { FilterOptions } from '@shared/FilterOptions'
import type { ResultsFilter } from '@shared/ResultsFilter'

export interface ResultsState {
  sessions: SessionSummary[]
  total: number
  detail: SessionDetail | null
  filterOptions: FilterOptions | null
  loading: boolean
  oauthLinked: boolean
  oauthMemberName: string | null
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
  const [state, setState] = useState<ResultsState>({
    sessions: [],
    total: 0,
    detail: null,
    filterOptions: null,
    loading: false,
    oauthLinked: false,
    oauthMemberName: null,
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
    setState((s) => ({ ...s, detail: null }))
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

  // Subscribe to WS messages relevant to results.
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
          // Re-query to pick up the new entry.
          query(filterRef.current)
          break
      }
    })
    return off
  }, [client, query])

  // Initial load when the view becomes active.
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

  return { state, filter, applyFilter, loadDetail, closeDetail, startOAuth, triggerFetch }
}
