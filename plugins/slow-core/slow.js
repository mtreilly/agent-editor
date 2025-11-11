#!/usr/bin/env node
// Slow core plugin: responds to a single JSON-RPC line after a configurable delay.
// Env: SLOW_DELAY_MS (default 1000)

const readline = require('readline')
const delay = Number(process.env.SLOW_DELAY_MS || 1000)

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
  setTimeout(() => respond(id, { ok: true, slow: true, delay }, null), delay)
})

rl.on('close', () => process.exit(0))

