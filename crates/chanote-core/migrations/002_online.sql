-- Online participation flag. `place_id` stays the group's place (e.g. 華待庵);
-- `online` records that the user joined remotely.
ALTER TABLE session ADD COLUMN online INTEGER NOT NULL DEFAULT 0;
