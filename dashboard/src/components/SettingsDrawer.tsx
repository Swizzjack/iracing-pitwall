import type { ReactNode } from 'react'

interface Props {
  open: boolean
  onClose: () => void
  title: string
  children: ReactNode
  variant?: 'card' | 'global'
}

export function SettingsDrawer({ open, onClose, title, children, variant }: Props) {
  const variantClass = variant === 'global' ? ' global' : ''
  return (
    <>
      {open && <div className={`settings-drawer-backdrop${variantClass}`} onClick={onClose} />}
      <div className={`settings-drawer${variantClass}${open ? ' open' : ''}`} aria-hidden={!open}>
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
