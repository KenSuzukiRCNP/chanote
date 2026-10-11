-- v0.4: a session is an event (会). Two axes replace the old `kind`:
--   form      what happened: temae (ordinary lesson / stand-alone procedures) | chaji | chakai | koshukai | '?'
--   practice  1 = rehearsal / lesson, 0 = the real thing
-- plus the kind of event (event_kind: vocab id of kind 'event-kind', '?' or NULL), my role
-- at the event (my_role / my_seat), and a session-level roster: session_person gains
-- seat / is_last / role_note and the roles guest | hanto | mizuya | host | kengaku next to
-- teacher. Temae rows without participants of their own inherit the roster (view below).
ALTER TABLE session ADD COLUMN form TEXT NOT NULL DEFAULT 'temae';
ALTER TABLE session ADD COLUMN event_kind TEXT;
ALTER TABLE session ADD COLUMN practice INTEGER NOT NULL DEFAULT 1;
ALTER TABLE session ADD COLUMN my_role TEXT;
ALTER TABLE session ADD COLUMN my_seat INTEGER;
ALTER TABLE session_person ADD COLUMN seat INTEGER;
ALTER TABLE session_person ADD COLUMN is_last INTEGER NOT NULL DEFAULT 0;
ALTER TABLE session_person ADD COLUMN role_note TEXT;

UPDATE session SET
  form     = CASE kind WHEN 'keiko' THEN 'temae' WHEN 'koshukai' THEN 'koshukai' ELSE '?' END,
  practice = CASE kind WHEN 'honban' THEN 0 ELSE 1 END;
ALTER TABLE session DROP COLUMN kind;

-- Effective participants of a temae: its own rows, or (when it has none) the session
-- roster minus teachers and the host.
CREATE VIEW temae_person_eff AS
  SELECT tp.temae_id, tp.person_id, tp.name_free, tp.role, tp.seat, tp.is_last, tp.role_note, 0 AS inherited
    FROM temae_person tp
  UNION ALL
  SELECT t.id, sp.person_id, sp.name_free,
         CASE sp.role WHEN 'guest' THEN 'kyaku' ELSE sp.role END,
         sp.seat, sp.is_last, sp.role_note, 1
    FROM temae t
    JOIN session_person sp ON sp.session_id = t.session_id
   WHERE sp.role IN ('guest', 'hanto', 'mizuya', 'kengaku')
     AND NOT EXISTS (SELECT 1 FROM temae_person x WHERE x.temae_id = t.id);
