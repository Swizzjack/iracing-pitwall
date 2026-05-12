import type { FilterOptions } from '@shared/FilterOptions'
import type { ResultsFilter } from '@shared/ResultsFilter'

interface Props {
  filter: ResultsFilter
  options: FilterOptions | null
  onChange: (patch: Partial<ResultsFilter>) => void
}

export function ResultsFilters({ filter, options, onChange }: Props) {
  return (
    <div className="results-filters">
      <select
        value={filter.trackId ?? ''}
        onChange={(e) => onChange({ trackId: e.target.value ? Number(e.target.value) : null })}
      >
        <option value="">All Tracks</option>
        {options?.tracks.map((t) => (
          <option key={t.trackId} value={t.trackId}>
            {t.trackConfig ? `${t.trackName} — ${t.trackConfig}` : t.trackName}
          </option>
        ))}
      </select>

      <select
        value={filter.carId ?? ''}
        onChange={(e) => onChange({ carId: e.target.value ? Number(e.target.value) : null })}
      >
        <option value="">All Cars</option>
        {options?.cars.map((c) => (
          <option key={c.carId} value={c.carId}>
            {c.carName}
          </option>
        ))}
      </select>

      <select
        value={filter.seriesId ?? ''}
        onChange={(e) => onChange({ seriesId: e.target.value ? Number(e.target.value) : null })}
      >
        <option value="">All Series</option>
        {options?.series.map((s) => (
          <option key={s.seriesId} value={s.seriesId}>
            {s.seriesName}
          </option>
        ))}
      </select>

      <select
        value={filter.eventType ?? ''}
        onChange={(e) => onChange({ eventType: e.target.value ? Number(e.target.value) : null })}
      >
        <option value="">All Types</option>
        {options?.eventTypes.map((et) => (
          <option key={et.eventType} value={et.eventType}>
            {et.eventTypeName}
          </option>
        ))}
      </select>

      <input
        type="date"
        value={filter.dateFrom ? new Date(filter.dateFrom).toISOString().slice(0, 10) : ''}
        onChange={(e) =>
          onChange({ dateFrom: e.target.value ? new Date(e.target.value).getTime() : null })
        }
        title="From date"
      />
      <span className="results-filter-sep">–</span>
      <input
        type="date"
        value={filter.dateTo ? new Date(filter.dateTo).toISOString().slice(0, 10) : ''}
        onChange={(e) =>
          onChange({
            dateTo: e.target.value ? new Date(e.target.value + 'T23:59:59Z').getTime() : null,
          })
        }
        title="To date"
      />

      {(filter.trackId || filter.carId || filter.seriesId || filter.eventType || filter.dateFrom || filter.dateTo) && (
        <button
          className="header-btn"
          onClick={() =>
            onChange({ trackId: null, carId: null, seriesId: null, eventType: null, dateFrom: null, dateTo: null })
          }
        >
          ✕ Clear
        </button>
      )}
    </div>
  )
}
