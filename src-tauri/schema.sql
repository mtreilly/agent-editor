PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS repo (
  id TEXT PRIMARY KEY,
  name TEXT,
  path TEXT NOT NULL UNIQUE,
  settings JSON,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS folder (
  id TEXT PRIMARY KEY,
  repo_id TEXT NOT NULL REFERENCES repo(id) ON DELETE CASCADE,
  parent_id TEXT REFERENCES folder(id) ON DELETE CASCADE,
  path TEXT NOT NULL,
  slug TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (repo_id, path)
);

CREATE TABLE IF NOT EXISTS doc (
  id TEXT PRIMARY KEY,
  repo_id TEXT NOT NULL REFERENCES repo(id) ON DELETE CASCADE,
  folder_id TEXT NOT NULL REFERENCES folder(id) ON DELETE CASCADE,
  slug TEXT NOT NULL,
  title TEXT NOT NULL,
  lang TEXT DEFAULT 'en',
  is_deleted INTEGER NOT NULL DEFAULT 0,
  current_version_id TEXT REFERENCES doc_version(id),
  size_bytes INTEGER DEFAULT 0,
  line_count INTEGER DEFAULT 0,
  backlink_count INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (repo_id, slug)
);

CREATE TABLE IF NOT EXISTS doc_blob (
  id TEXT PRIMARY KEY,
  content BLOB NOT NULL,
  encoding TEXT DEFAULT 'utf8',
  mime TEXT DEFAULT 'text/markdown',
  size_bytes INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS doc_version (
  id TEXT PRIMARY KEY,
  doc_id TEXT NOT NULL REFERENCES doc(id) ON DELETE CASCADE,
  blob_id TEXT NOT NULL REFERENCES doc_blob(id) ON DELETE RESTRICT,
  author TEXT,
  message TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  hash TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS link (
  id TEXT PRIMARY KEY,
  repo_id TEXT NOT NULL REFERENCES repo(id) ON DELETE CASCADE,
  from_doc_id TEXT NOT NULL REFERENCES doc(id) ON DELETE CASCADE,
  to_doc_id TEXT,
  to_slug TEXT NOT NULL,
  type TEXT NOT NULL CHECK (type IN ('wiki','url','heading','file')),
  line_start INTEGER,
  line_end INTEGER,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (from_doc_id, to_slug, line_start, line_end)
);

CREATE TABLE IF NOT EXISTS provenance (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  source TEXT NOT NULL CHECK (source IN ('fs','ai','import','plugin')),
  meta JSON,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS scan_job (
  id TEXT PRIMARY KEY,
  repo_id TEXT NOT NULL REFERENCES repo(id) ON DELETE CASCADE,
  status TEXT NOT NULL CHECK (status IN ('queued','running','success','error','partial')),
  stats JSON,
  started_at TEXT NOT NULL DEFAULT (datetime('now')),
  finished_at TEXT,
  error TEXT
);

CREATE TABLE IF NOT EXISTS ai_trace (
  id TEXT PRIMARY KEY,
  repo_id TEXT NOT NULL REFERENCES repo(id) ON DELETE CASCADE,
  doc_id TEXT,
  anchor_id TEXT,
  provider TEXT NOT NULL,
  request JSON NOT NULL,
  response JSON,
  input_tokens INTEGER,
  output_tokens INTEGER,
  cost_usd REAL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS plugin (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  version TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('ui','core')),
  manifest JSON NOT NULL,
  permissions JSON NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  installed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS plugin_event (
  id TEXT PRIMARY KEY,
  plugin_id TEXT NOT NULL REFERENCES plugin(id) ON DELETE CASCADE,
  type TEXT NOT NULL,
  payload JSON,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE VIRTUAL TABLE IF NOT EXISTS doc_fts USING fts5(
  title, body, slug, repo_id, content='doc', content_rowid='rowid',
  tokenize='unicode61 remove_diacritics 2'
);
