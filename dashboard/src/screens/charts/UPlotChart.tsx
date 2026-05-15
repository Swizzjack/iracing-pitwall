import { useEffect, useLayoutEffect, useRef } from 'react'
import uPlot from 'uplot'
import 'uplot/dist/uPlot.min.css'

interface Props {
  data: uPlot.AlignedData
  options: Omit<uPlot.Options, 'width' | 'height'>
  height?: number
}

export function UPlotChart({ data, options, height = 260 }: Props) {
  const containerRef = useRef<HTMLDivElement>(null)
  const uRef = useRef<uPlot | null>(null)
  const dataRef = useRef(data)
  dataRef.current = data

  // Create/destroy when options (series structure) changes — use JSON key as stability signal.
  const seriesKey = options.series?.length ?? 0

  useLayoutEffect(() => {
    const el = containerRef.current
    if (!el) return

    const width = el.clientWidth || 600
    const u = new uPlot({ ...options, width, height }, dataRef.current, el)
    uRef.current = u

    const ro = new ResizeObserver(([entry]) => {
      if (uRef.current) {
        uRef.current.setSize({ width: entry.contentRect.width, height })
      }
    })
    ro.observe(el)

    return () => {
      ro.disconnect()
      u.destroy()
      uRef.current = null
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [seriesKey, height])

  // Update data without recreating
  useEffect(() => {
    uRef.current?.setData(data)
  }, [data])

  return <div ref={containerRef} className="uplot-container" />
}
