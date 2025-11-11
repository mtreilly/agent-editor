import * as React from 'react'

type HintProps = {
  id: string
  children: React.ReactNode
  className?: string
}

export function Hint({ id, children, className }: HintProps) {
  return (
    <span id={id} className={className ?? 'text-xs text-gray-600'}>
      {children}
    </span>
  )
}

