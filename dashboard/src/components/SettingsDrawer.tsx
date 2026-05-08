import type { ReactNode } from 'react'

interface Props {
  open: boolean
  onClose: () => void
  title: string
  children: ReactNode
}

export function SettingsDrawer({ open, onClose, title, children }: Props) {
  return (
    <>
      {open && <div className="settings-drawer-backdrop" onClick={onClose} />}
      <div className={`settings-drawer${open ? ' open' : ''}`} aria-hidden={!open}>
        <div className="settings-drawer-header">
          <span className="settings-drawer-title">{title}</span>
          <button className="settings-drawer-close" onClick={onClose} title="Close">✕</button>
        </div>
        <div className="settings-drawer-body">
          {children}
        </div>
      </div>
    </>
  )
}
