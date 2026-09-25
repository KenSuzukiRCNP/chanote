-- chanote schema v1. See docs/chanote-design.md §2.
-- Reserved vocab ids used in axis columns: '?' = unknown, '-' = not applicable.

CREATE TABLE vocab (
  id        TEXT PRIMARY KEY,
  kind      TEXT NOT NULL,            -- temae | place | person | tana | hearth | tool | school
  ja        TEXT NOT NULL,
  romaji    TEXT,
  en        TEXT,
  parent_id TEXT REFERENCES vocab(id),
  sort      INTEGER,
  origin    TEXT NOT NULL DEFAULT 'user',   -- 'pack:<school>' | 'user'
  archived  INTEGER NOT NULL DEFAULT 0,
  meta      TEXT                      -- JSON
);
CREATE INDEX vocab_kind ON vocab(kind, archived, sort);

CREATE TABLE session (
  id         INTEGER PRIMARY KEY,
  date       TEXT,                    -- YYYY-MM-DD (day part may be 01 when prec < day)
  date_prec  TEXT NOT NULL DEFAULT 'day',  -- day | month | year | unknown
  place_id   TEXT REFERENCES vocab(id),
  place_note TEXT,
  kind       TEXT NOT NULL DEFAULT 'keiko', -- keiko | koshukai | honban
  note       TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX session_date ON session(date);

CREATE TABLE session_person (
  id         INTEGER PRIMARY KEY,
  session_id INTEGER NOT NULL REFERENCES session(id) ON DELETE CASCADE,
  person_id  TEXT REFERENCES vocab(id),
  name_free  TEXT,
  role       TEXT NOT NULL,           -- teacher | guest
  ord        INTEGER
);
CREATE INDEX session_person_s ON session_person(session_id);

CREATE TABLE temae (
  id             INTEGER PRIMARY KEY,
  session_id     INTEGER NOT NULL REFERENCES session(id) ON DELETE CASCADE,
  ord            INTEGER NOT NULL,
  time           TEXT,
  temae_id       TEXT REFERENCES vocab(id),
  temae_alias_id TEXT,
  temae_free     TEXT,
  host_id        TEXT REFERENCES vocab(id),
  host_free      TEXT,
  teacher_id     TEXT REFERENCES vocab(id),
  my_role        TEXT,                -- teishu | hanto | mizuya | kyaku | shokyaku | kengaku | -
  my_role_note   TEXT,
  my_seat        INTEGER,
  hearth         TEXT,
  katte          TEXT,
  room           TEXT,
  tea            TEXT,
  tana_id        TEXT,
  tana_free      TEXT,
  kinin          TEXT,
  note           TEXT
);
CREATE INDEX temae_s ON temae(session_id, ord);
CREATE INDEX temae_t ON temae(temae_id);

CREATE TABLE temae_person (
  id        INTEGER PRIMARY KEY,
  temae_id  INTEGER NOT NULL REFERENCES temae(id) ON DELETE CASCADE,
  person_id TEXT REFERENCES vocab(id),
  name_free TEXT,
  role      TEXT NOT NULL,            -- kyaku | hanto | mizuya | kengaku
  seat      INTEGER,
  is_last   INTEGER NOT NULL DEFAULT 0,
  role_note TEXT
);
CREATE INDEX temae_person_t ON temae_person(temae_id);

CREATE TABLE temae_tsukimono (
  temae_id  INTEGER NOT NULL REFERENCES temae(id) ON DELETE CASCADE,
  tsukimono TEXT NOT NULL,
  PRIMARY KEY (temae_id, tsukimono)
);

CREATE TABLE temae_tool (
  id           INTEGER PRIMARY KEY,
  temae_id     INTEGER NOT NULL REFERENCES temae(id) ON DELETE CASCADE,
  ord          INTEGER,
  tool_id      TEXT REFERENCES vocab(id),
  name_free    TEXT,
  external_ref TEXT
);
CREATE INDEX temae_tool_t ON temae_tool(temae_id);

CREATE TABLE attachment (
  id         INTEGER PRIMARY KEY,
  session_id INTEGER REFERENCES session(id) ON DELETE CASCADE,
  temae_id   INTEGER REFERENCES temae(id) ON DELETE CASCADE,
  path       TEXT NOT NULL,
  caption    TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE setting (
  key   TEXT PRIMARY KEY,
  value TEXT
);

CREATE TABLE pack_installed (
  pack_id      TEXT PRIMARY KEY,      -- 'urasenke'
  pack_version TEXT NOT NULL,
  installed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
