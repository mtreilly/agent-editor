import { $node, $mark, $command, $inputRule } from '@milkdown/utils'

export const wikiLink = $node('wiki_link', () => ({
  inline: true,
  group: 'inline',
  atom: true,
  selectable: true,
  attrs: { slug: {}, alias: { default: null }, heading: { default: null } },
  toDOM: (node) => ['a', { 'data-slug': node.attrs.slug, class: 'wiki text-blue-600 underline' }, node.attrs.alias || node.attrs.slug],
  parseDOM: [
    {
      tag: 'a.wiki',
      getAttrs: (el) => ({ slug: (el as Element).getAttribute('data-slug') }),
    },
  ],
}))

export const anchorMark = $mark('anchor', () => ({
  inclusive: false,
  attrs: { id: {} },
  parseDOM: [{ tag: 'span[data-anchor-id]' }],
  toDOM: (mark) => ['span', { 'data-anchor-id': mark.attrs.id }, 0],
}))

export const insertWikiLink = $command('insertWikiLink', () => (slug: string) => (state, dispatch) => {
  const { from, to } = state.selection
  const node = state.schema.nodes['wiki_link'].create({ slug })
  if (dispatch) dispatch(state.tr.replaceRangeWith(from, to, node))
  return true
})

export const wikiInputRule = $inputRule(/\[\[([^\]]+)\]\]$/, (state, match, start, end) => {
  const [_, inner] = match as unknown as [string, string]
  const [slug, alias] = inner.split('|')
  return state.tr.replaceWith(start, end, state.schema.nodes['wiki_link'].create({ slug, alias: alias || null }))
})

