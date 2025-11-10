import * as React from 'react'
import { Milkdown, useEditor } from '@milkdown/react'
import { Editor as MilkEditor, rootCtx, defaultValueCtx } from '@milkdown/core'
import { gfm } from '@milkdown/preset-gfm'
import { listener, listenerCtx } from '@milkdown/plugin-listener'
import { wikiLink, anchorMark } from '../../../src/editor/schema/wiki'

type Props = {
  value: string
  onChange?: (md: string) => void
}

export function Editor({ value, onChange }: Props) {
  useEditor((root) => {
    return MilkEditor.make()
      .config((ctx) => {
        ctx.set(rootCtx, root)
        ctx.set(defaultValueCtx, value)
        const l = ctx.get(listenerCtx)
        l.markdownUpdated((_, md) => onChange?.(md))
      })
      .use(gfm)
      .use(listener)
      .use(wikiLink)
      .use(anchorMark)
  }, [value])

  return <Milkdown />
}

