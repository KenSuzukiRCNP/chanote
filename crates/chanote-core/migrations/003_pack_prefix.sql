-- Pack-scoped ids: temae/temae-cat/tana rows from a school pack are renamed from
-- `temae.<id>` to `temae.<pack>.<id>` so that two schools never collide. The rewrite
-- touches JSON and record columns, so it is done in Rust (db.rs, data_migration_003).
SELECT 1;
