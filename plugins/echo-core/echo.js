#!/usr/bin/env node
// Simple echo core plugin: reads lines from stdin and writes JSON responses to stdout
const readline = require('readline')
const rl = readline.createInterface({ input: process.stdin, output: process.stdout, terminal: false })
rl.on('line', (line) => {
  const trimmed = (line || '').trim()
  if (!trimmed) return
  const msg = { ok: true, echo: trimmed }
  process.stdout.write(JSON.stringify(msg) + '\n')
})

