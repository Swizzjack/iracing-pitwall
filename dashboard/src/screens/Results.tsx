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
  const { state, filter, applyFilter, loadDetail, closeDetail, startOAuth } = useResults(
    client,
    active,
  )

  if (state.detail) {
    return <ResultDetail detail={state.detail} onClose={closeDetail} />
  }

  return (
    <div className="results-screen">
      <div className="results-toolbar">
        <div className="results-oauth">
          {state.oauthLinked ? (
            <span className="oauth-status oauth-linked">
              ● {state.oauthMemberName ?? 'Linked'}
            </span>
          ) : (
            <button className="header-btn" onClick={startOAuth}>
              Connect iRacing Account
            </button>
          )}
        </div>
        <ResultsFilters filter={filter} options={state.filterOptions} onChange={applyFilter} />
        {state.loading && <span className="results-loading">Loading…</span>}
        <span className="results-count">{state.total} result{state.total !== 1 ? 's' : ''}</span>
      </div>

      {!state.oauthLinked && state.sessions.length === 0 ? (
        <div className="results-empty">
          <p>Connect your iRacing account to start fetching results automatically after each session.</p>
          <button className="header-btn" onClick={startOAuth}>Connect iRacing Account</button>
        </div>
      ) : (
        <ResultsTable sessions={state.sessions} onSelect={loadDetail} />
      )}
    </div>
  )
}
