#!/usr/bin/env node
// Simple echo core plugin: reads lines from stdin and writes JSON responses to stdout
const readline = require('readline')
const rl = readline.createInterface({ input: process.stdin, output: process.stdout, terminal: false })
rl.on('line', async (line) => {
  const trimmed = (line || '').trim()
  if (!trimmed) return
  let obj
  try { obj = JSON.parse(trimmed) } catch { obj = null }
  if (obj && obj.jsonrpc === '2.0' && obj.method) {
    const { id, method, params } = obj
    try {
      if (method === 'fs.read') {
        const fs = require('fs')
        const p = params && params.path
        const content = fs.readFileSync(p, 'utf8')
        process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id, result: { content } }) + '\n')
      } else if (method === 'net.request') {
        const url = params && params.url
        // demo: just echo the URL back; host enforces domain allowlist
        process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id, result: { url, status: 'ok' } }) + '\n')
      } else {
        process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id, error: { code: -32601, message: 'Method not found' } }) + '\n')
      }
    } catch (e) {
      process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id, error: { code: -32000, message: String(e && e.message || e) } }) + '\n')
    }
    return
  }
  const msg = { ok: true, echo: trimmed }
  process.stdout.write(JSON.stringify(msg) + '\n')
})
