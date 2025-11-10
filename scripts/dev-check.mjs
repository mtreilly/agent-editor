#!/usr/bin/env node
import http from 'node:http'

const payload = JSON.stringify({ jsonrpc: '2.0', id: '1', method: 'repos_list', params: {} })
const req = http.request(
  { hostname: '127.0.0.1', port: 35678, path: '/rpc', method: 'POST', headers: { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(payload) } },
  (res) => {
    let data = ''
    res.on('data', (c) => (data += c))
    res.on('end', () => {
      try {
        const out = JSON.parse(data)
        if (out.error) {
          console.error('RPC error:', out.error)
          process.exitCode = 2
        } else {
          console.log('RPC ok:', JSON.stringify(out.result))
        }
      } catch (e) {
        console.error('Invalid response:', data)
        process.exitCode = 3
      }
    })
  }
)
req.on('error', (e) => {
  console.error('Connection failed:', e.message)
  process.exitCode = 1
})
req.write(payload)
req.end()

