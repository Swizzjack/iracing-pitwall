import type { WsClient } from '../ws/Client'
import { useResults } from './useResults'
import { ResultsFilters } from './ResultsFilters'
import { ResultsTable } from './ResultsTable'
import { ResultDetail } from './ResultDetail'

interface Props {
  client: WsClient
  active: boolean
}

export function Results({ client, active }: Props) {
  const {
    state,
    filter,
    applyFilter,
    loadDetail,
    closeDetail,
    loadLaps,
    closeLaps,
    loadChartLaps,
  } = useResults(client, active)

  if (state.detail) {
    return (
      <ResultDetail
        detail={state.detail}
        laps={state.laps}
        lapsCarIdx={state.lapsCarIdx}
        chartLaps={state.chartLaps}
        onClose={closeDetail}
        onLoadLaps={loadLaps}
        onCloseLaps={closeLaps}
        onLoadChartLaps={loadChartLaps}
      />
    )
  }

  return (
    <div className="results-screen">
      <div className="results-toolbar">
        <ResultsFilters filter={filter} options={state.filterOptions} onChange={applyFilter} />
        {state.loading && <span className="results-loading">Loading…</span>}
        <span className="results-count">{state.total} result{state.total !== 1 ? 's' : ''}</span>
      </div>

      {state.sessions.length === 0 ? (
        <div className="results-empty">
          <p>Results from finished sessions appear here automatically.</p>
        </div>
      ) : (
        <ResultsTable sessions={state.sessions} onSelect={loadDetail} />
      )}
    </div>
  )
}
