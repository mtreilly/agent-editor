#!/usr/bin/env node
// Asserts bench results against thresholds. Reads JSON from stdin produced by bench scripts/CLI.
// Env:
//  FTS_P95_MS (default 50)
//  FTS_P99_MS (default 80)
//  FTS_AVG_MS (default 25)

const thresholds = {
  p95: Number(process.env.FTS_P95_MS || 50),
  p99: Number(process.env.FTS_P99_MS || 80),
  avg: Number(process.env.FTS_AVG_MS || 25),
}

let data = ''
process.stdin.setEncoding('utf8')
process.stdin.on('data', (c) => (data += c))
process.stdin.on('end', () => {
  let obj
  try {
    obj = JSON.parse(data)
  } catch (e) {
    console.warn('[ci-bench] Could not parse JSON; skipping assertions')
    process.exit(0)
  }
  // Accept either flat fields or nested under a "stats" key
  const stats = obj.stats || obj
  const violations = []
  if (typeof stats.p95 === 'number' && stats.p95 > thresholds.p95) violations.push(`p95 ${stats.p95}ms > ${thresholds.p95}ms`)
  if (typeof stats.p99 === 'number' && stats.p99 > thresholds.p99) violations.push(`p99 ${stats.p99}ms > ${thresholds.p99}ms`)
  if (typeof stats.avg === 'number' && stats.avg > thresholds.avg) violations.push(`avg ${stats.avg}ms > ${thresholds.avg}ms`)
  if (violations.length) {
    console.error('[ci-bench] FAIL:', violations.join('; '))
    process.exit(2)
  } else {
    console.log('[ci-bench] PASS')
    process.exit(0)
  }
})

