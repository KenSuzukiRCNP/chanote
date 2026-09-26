-- Online participation flag. `place_id` stays the group's place (the tea room the group meets in);
-- `online` records that the user joined remotely.
ALTER TABLE session ADD COLUMN online INTEGER NOT NULL DEFAULT 0;
