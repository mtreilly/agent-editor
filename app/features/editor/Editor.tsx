import * as React from 'react'
import { Milkdown, useEditor } from '@milkdown/react'
import { Editor as MilkEditor, rootCtx, defaultValueCtx, editorViewCtx } from '@milkdown/core'
import { gfm } from '@milkdown/preset-gfm'
import { listener, listenerCtx } from '@milkdown/plugin-listener'
import { wikiLink, anchorMark } from '../../../src/editor/schema/wiki'

type Props = {
  value: string
  onChange?: (md: string) => void
  docId?: string
  onReady?: (api: { insertAnchor: (id?: string) => { id: string; line: number } | null }) => void
}

export function Editor({ value, onChange, docId, onReady }: Props) {
  const apiRef = React.useRef<{ insertAnchor: (id?: string) => { id: string; line: number } | null } | null>(null)
  useEditor((root) => {
    return MilkEditor.make()
      .config((ctx) => {
        ctx.set(rootCtx, root)
        ctx.set(defaultValueCtx, value)
        const l = ctx.get(listenerCtx)
        l.markdownUpdated((_, md) => onChange?.(md))

        const insertAnchor = (id?: string) => {
          const view = ctx.get(editorViewCtx)
          const { state, dispatch } = view
          const pos = state.selection.from
          const beforeText = state.doc.textBetween(0, pos, '\n', '\n')
          const line = beforeText.split('\n').length
          const schema = state.schema
          const markType = schema.marks['anchor']
          if (!markType) return null
          const anchorId = id || `anc_${docId || 'doc'}_${line}_${Date.now().toString(36).slice(-4)}`
          const tr = state.tr.insertText('\u200B', pos, pos)
          tr.addMark(pos, pos + 1, markType.create({ id: anchorId }))
          dispatch(tr)
          return { id: anchorId, line }
        }
        apiRef.current = { insertAnchor }
        onReady?.(apiRef.current)
      })
      .use(gfm)
      .use(listener)
      .use(wikiLink)
      .use(anchorMark)
  }, [value])

  const onClick = React.useCallback((e: React.MouseEvent) => {
    const el = e.target as HTMLElement
    if (el && el.tagName === 'A' && el.classList.contains('wiki')) {
      e.preventDefault()
      const slug = (el as HTMLAnchorElement).getAttribute('data-slug') || ''
      // Defer navigation to parent via a custom event
      const evt = new CustomEvent('wiki:navigate', { detail: { slug } })
      el.dispatchEvent(evt)
    }
  }, [])

  return (
    <div onClick={onClick}>
      <Milkdown />
    </div>
  )
}
