import { useState, useEffect, useRef } from 'react'
import { REGISTRY } from './registry'

interface Props {
  editing: boolean
  visible: string[]
  onToggleEdit: () => void
  onAdd: (id: string) => void
  onReset: () => void
}

export function EditToolbar({ editing, visible, onToggleEdit, onAdd, onReset }: Props) {
  const [addOpen, setAddOpen] = useState(false)
  const menuRef = useRef<HTMLDivElement>(null)
  const hidden = REGISTRY.filter((d) => !visible.includes(d.id))

  // Close dropdown on outside click
  useEffect(() => {
    if (!addOpen) return
    function handleClick(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setAddOpen(false)
      }
    }
    document.addEventListener('mousedown', handleClick)
    return () => document.removeEventListener('mousedown', handleClick)
  }, [addOpen])

  function handleToggleEdit() {
    if (editing) setAddOpen(false)
    onToggleEdit()
  }

  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
      {editing && hidden.length > 0 && (
        <div ref={menuRef} style={{ position: 'relative' }}>
          <button
            className="header-btn"
            onClick={() => setAddOpen((o) => !o)}
            title="Add widget"
          >
            +
          </button>
          {addOpen && (
            <div className="add-menu">
              {hidden.map((d) => (
                <button
                  key={d.id}
                  className="add-menu-item"
                  onClick={() => { onAdd(d.id); setAddOpen(false) }}
                >
                  {d.title}
                </button>
              ))}
            </div>
          )}
        </div>
      )}
      {editing && (
        <button
          className="header-btn header-btn-danger"
          onClick={() => { if (confirm('Reset layout to defaults?')) onReset() }}
          title="Reset layout"
        >
          ↺
        </button>
      )}
      <button
        className={`header-btn ${editing ? 'header-btn-active' : ''}`}
        onClick={handleToggleEdit}
        title={editing ? 'Layout sperren' : 'Layout bearbeiten'}
      >
        ✎
      </button>
    </div>
  )
}
