-- ─────────────────────────────────────────
-- GAMES
-- Central registry of all games known to Wither.
-- source: 'steam' | 'custom'
-- drm_type: 'steam' | 'none'
-- launch_method: 'steam_protocol' | 'executable'
-- ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS games (
  id                TEXT PRIMARY KEY,
  name              TEXT NOT NULL,
  source            TEXT NOT NULL,
  drm_type          TEXT NOT NULL DEFAULT 'none',
  launch_method     TEXT NOT NULL,
  executable_path   TEXT,
  launch_args       TEXT,
  steam_app_id      INTEGER,
  cover_url         TEXT,
  background_url    TEXT,
  genre             TEXT,
  developer         TEXT,
  release_year      INTEGER,
  steam_playtime_s  INTEGER DEFAULT 0,
  is_installed      INTEGER DEFAULT 1,
  is_favourite      INTEGER DEFAULT 0,
  added_at          INTEGER NOT NULL,
  last_synced_at    INTEGER
);

-- ─────────────────────────────────────────
-- SESSIONS
-- Every individual play session tracked by Wither.
-- duration_s is the authoritative playtime source.
-- ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS sessions (
  id            TEXT PRIMARY KEY,
  game_id       TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
  started_at    INTEGER NOT NULL,
  ended_at      INTEGER,
  duration_s    INTEGER,
  was_crashed   INTEGER DEFAULT 0
);

-- ─────────────────────────────────────────
-- COLLECTIONS
-- User-created playlists / groups of games.
-- ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS collections (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  created_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS collection_games (
  collection_id  TEXT REFERENCES collections(id) ON DELETE CASCADE,
  game_id        TEXT REFERENCES games(id) ON DELETE CASCADE,
  added_at       INTEGER NOT NULL,
  PRIMARY KEY (collection_id, game_id)
);

-- ─────────────────────────────────────────
-- SETTINGS
-- Key-value store for user preferences.
-- ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS settings (
  key    TEXT PRIMARY KEY,
  value  TEXT NOT NULL
);

-- ─────────────────────────────────────────
-- STEAM REMOVED GAMES TRACKING
-- Prevents manually removed games from reappearing
-- ─────────────────────────────────────────

CREATE TABLE IF NOT EXISTS steam_removed_games (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    steam_app_id INTEGER NOT NULL UNIQUE,
    removed_at INTEGER NOT NULL,
    reason TEXT DEFAULT 'manual'
);

-- Default settings
INSERT OR IGNORE INTO settings (key, value) VALUES
  ('steam_api_key', ''),
  ('steam_user_id', ''),
  ('steam_username', ''),
  ('steam_avatar_url', ''),
  ('steam_path', ''),
  ('sync_interval_minutes', '10'),
  ('launch_on_startup', 'false'),
  ('close_to_tray', 'true'),
  ('steamgriddb_api_key', ''),
  ('accent_color', '#D62828'),
  ('store_country_code', 'us'),
  ('store_language', 'english');

-- ─────────────────────────────────────────
-- IMAGE CACHE
-- Tracks locally cached cover/background images.
-- ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS image_cache (
  url         TEXT PRIMARY KEY,
  local_path  TEXT NOT NULL,
  cached_at   INTEGER NOT NULL
);

-- ─────────────────────────────────────────
-- STORE CACHE
-- Caches Steam Store API responses (Module 11).
-- ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS store_cache (
  key         TEXT PRIMARY KEY,      -- e.g. 'featured:it', 'appdetails:570:it'
  value       TEXT NOT NULL,         -- JSON blob
  cached_at   INTEGER NOT NULL
);

-- ─────────────────────────────────────────
-- INDEXES
-- ─────────────────────────────────────────
CREATE INDEX IF NOT EXISTS idx_sessions_game_id ON sessions(game_id);
CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at);
CREATE INDEX IF NOT EXISTS idx_games_source ON games(source);
CREATE INDEX IF NOT EXISTS idx_games_last_played ON sessions(started_at DESC);

-- ─────────────────────────────────────────
-- VIEW: Total playtime per game
-- ─────────────────────────────────────────
CREATE VIEW IF NOT EXISTS v_playtime AS
SELECT
  game_id,
  COALESCE(SUM(duration_s), 0) AS total_s,
  MAX(started_at)               AS last_played_at,
  COUNT(*)                      AS session_count
FROM sessions
WHERE ended_at IS NOT NULL
GROUP BY game_id;

-- ─────────────────────────────────────────
-- VIEW: Games enriched with playtime
-- ─────────────────────────────────────────
CREATE VIEW IF NOT EXISTS v_games_full AS
SELECT
  g.*,
  COALESCE(p.total_s, 0)        AS wither_playtime_s,
  COALESCE(p.last_played_at, 0) AS last_played_at,
  COALESCE(p.session_count, 0)  AS session_count
FROM games g
LEFT JOIN v_playtime p ON p.game_id = g.id;
