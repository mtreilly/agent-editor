// Minimal HTML sanitizer for snippets coming from SQLite FTS snippet()
// Removes script/style tags and event handler attributes. Not a full DOMPurify replacement
export function sanitizeHtml(input: string): string {
  if (!input) return ''
  let out = input
  // Strip script/style tags and their content
  out = out.replace(/<\s*(script|style)[^>]*>[\s\S]*?<\s*\/\s*\1\s*>/gi, '')
  // Remove on* attributes (onclick, onload, etc.)
  out = out.replace(/\s+on[a-z]+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)/gi, '')
  // Remove javascript: urls
  out = out.replace(/href\s*=\s*("|')\s*javascript:[^"']*(\1)/gi, 'href="#"')
  return out
}

