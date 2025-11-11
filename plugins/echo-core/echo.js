#!/usr/bin/env node
// Simple echo core plugin: reads JSON-RPC lines on stdin and writes responses on stdout.
// Supported methods (stubbed): fs.read, net.request, db.query, db.writeInsert, ai.invoke

const readline = require('readline')

const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity })

function respond(id, result, error) {
  const out = error ? { jsonrpc: '2.0', id, error: { code: -32000, message: String(error) } } : { jsonrpc: '2.0', id, result }
  process.stdout.write(JSON.stringify(out) + '\n')
}

rl.on('line', (line) => {
  if (!line.trim()) return
  let msg
  try { msg = JSON.parse(line) } catch { return respond(null, null, 'invalid_json') }
  const id = msg.id ?? null
  const method = msg.method || ''
  const params = msg.params || {}
  try {
    switch (true) {
      case method.startsWith('fs.read'):
        return respond(id, { ok: true, kind: 'fs.read', path: params.path || '' })
      case method.startsWith('net.request'):
        return respond(id, { ok: true, kind: 'net.request', url: params.url || '' })
      case method.startsWith('db.query'):
        return respond(id, { ok: true, kind: 'db.query', rows: [] })
      case method.startsWith('db.write'):
        return respond(id, { ok: true, kind: 'db.write', affected: 1 })
      case method.startsWith('ai.invoke'):
        return respond(id, { ok: true, kind: 'ai.invoke', text: 'echo' })
      default:
        return respond(id, { ok: true, echo: { method, params } })
    }
  } catch (e) {
    return respond(id, null, e.message || String(e))
  }
})

rl.on('close', () => process.exit(0))

