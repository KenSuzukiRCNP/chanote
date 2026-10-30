-- v0.5.1: folder sync. Records get a global id (sessions and attachments; vocab ids are
-- already global strings), and the sync bookkeeping tables are added. See sync.rs.
ALTER TABLE session ADD COLUMN uid TEXT;
UPDATE session SET uid = lower(hex(randomblob(16))) WHERE uid IS NULL;
CREATE UNIQUE INDEX session_uid ON session(uid);

ALTER TABLE attachment ADD COLUMN uid TEXT;
UPDATE attachment SET uid = lower(hex(randomblob(16))) WHERE uid IS NULL;
CREATE UNIQUE INDEX attachment_uid ON attachment(uid);

-- current revision of every synced record ("<time>|<counter>|<device>"); deleted = tombstone
CREATE TABLE sync_rev (
  kind    TEXT NOT NULL,          -- session | vocab | attachment
  key     TEXT NOT NULL,          -- session uid | vocab id | attachment uid
  rev     TEXT NOT NULL,
  deleted INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (kind, key)
);
-- local changes not yet written to the sync folder
CREATE TABLE sync_outbox (seq INTEGER PRIMARY KEY, event TEXT NOT NULL);
-- last log file applied, per remote device
CREATE TABLE sync_seen (device TEXT PRIMARY KEY, file_seq INTEGER NOT NULL);
-- remote events that could not be applied yet (e.g. a record referring to a missing name)
CREATE TABLE sync_pending (id INTEGER PRIMARY KEY, event TEXT NOT NULL, error TEXT);
-- concurrent edits: the version that lost, kept so nothing disappears silently
CREATE TABLE sync_conflict (
  id       INTEGER PRIMARY KEY,
  kind     TEXT NOT NULL,
  key      TEXT NOT NULL,
  label    TEXT,
  at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  kept_rev TEXT,
  lost_rev TEXT,
  lost     TEXT NOT NULL,             -- the losing event (JSON)
  resolved INTEGER NOT NULL DEFAULT 0
);
