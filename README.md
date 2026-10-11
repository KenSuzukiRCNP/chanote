# chanote

**A practice diary for the Japanese tea ceremony (chadō).**
日本語版は [README.ja.md](README.ja.md) をご覧ください。

You practise every week. You do a temae, you watch others, you sit as a guest, a teacher
corrects something. Months later you cannot remember when you last did *Karamono* with a
ro hearth, which shelf you used, or who was main guest that day. chanote keeps that diary
for you and turns it into something you can look back on.

<p align="center">
  <img src="docs/screenshots/home.png" width="49%" alt="Practice records">
  <img src="docs/screenshots/stats-coverage.png" width="49%" alt="Coverage statistics">
</p>

## What it does

**Records that look like your notebook.** One entry per practice day: date, place, teacher,
and one line per temae — the procedure, who was host, and what *you* did (host, main guest,
guest, assistant, or just watching). Add as much or as little as you remember: hearth,
shelf, thick or thin tea, orientation, room, seating order, utensils, a note, photos.
A date can be "sometime in March 2019"; a shelf can be "there was one, I forget which".

**Chaji and chakai, not just lessons.** A record is an *event*: an ordinary lesson, a chaji
(with its kind — 正午, 夜咄, 朝茶事 …), a chakai (初釜, 月釜 …) or a workshop, and a tick says whether
it was practice or the real thing. Write the guest list, assistant and host once for the whole
event; every temae in it inherits them unless you change one. Then ask: how many chaji have I
done, how often was I main guest, last guest, assistant or in the back room?

**Your own names for things.** "Nagaita koicha" or "usucha with the travel chest" — register
the names you actually use and chanote records them as the underlying procedure plus its
settings, so statistics still add up.

**Find it again.** Filter the diary by procedure or a whole category (all of *shikaden*,
say), period, place, teacher, people who were there, utensils used, or words in your
notes — and switch on the notes to re-read your teacher's remarks before the next lesson.

**Statistics that answer real questions.**
- *Coverage* — every procedure in the school's curriculum, how often you have done it, when
  last, grouped by category (konarai, shikaden, okuden …) with a progress bar per category.
- *Not done recently* — what to ask your teacher for next, filtered to the current season.
- *Timeline* — monthly practice by ro / furo season.
- *Cross tables* — temae × shelf, temae × hearth, place × teacher, anything × anything.
- *People and places* — by teacher, by place, whose temae you watched, who sat with you.
- *Events* — chaji and chakai by kind, practice vs. real, and your role at each.

<p align="center">
  <img src="docs/screenshots/home-filter.png" width="49%" alt="Filtering records">
  <img src="docs/screenshots/stats-timeline.png" width="49%" alt="Timeline">
</p>

**Utensils with a memory.** Categories (tea bowl, thin-tea container, chaire, shifuku,
scoop, water jar, pitcher), a free note for provenance and box inscriptions, and pairs —
pick a chaire and its shifuku comes along.

**Yours, on your machine.** Everything lives in one SQLite file plus a photo folder on your
computer. No account, no server, no tracking. Export to JSON (complete) or CSV (one row per
temae, for Excel or pandas), make a zip backup with photos, restore it on another Mac.
Several people can share one computer, each with their own diary — and a friend's export
can be imported *as your own* diary, with their guest entries becoming your roles.

**Japanese and English.** The interface is available in both. Tea terms are shown in
Japanese, Japanese + romaji, or romaji only, with an English gloss on hover.

## Which school?

The built-in list of procedures is **Urasenke** (平点前, 小習十六ヶ条, 炭手前, 茶箱, 四ヶ伝,
奥伝, 七事式 …). It is data, not code, so if you study another school you can already
rename, hide and add procedures in the list editor and keep a perfectly good diary.

Proper support for other schools — Omotesenke, Mushakōji-senke, Edo-senke, Enshū and
others — is planned as **school packs**: one YAML file per school that describes its
procedures, shelves and defaults. The author practises Urasenke and does not know the
other traditions well enough to write those files alone. If you do, **help is very
welcome** — see [docs/school-packs.md](docs/school-packs.md) for the format; a pack is a
few hundred lines of plain text and needs no programming.

## Install

Download the latest release from the **[Releases](../../releases)** page.

- **macOS**: open the `.dmg` and drag *chanote* into Applications. Pick `aarch64` for Apple
  Silicon (M1 and later) or `x64` for Intel. The first time, right-click the app and choose
  *Open* (the app is not yet signed with Apple; after that it opens normally). If macOS says
  the app is damaged, run `xattr -cr /Applications/chanote.app` once in Terminal.
- **Windows**: run the `.exe` installer.
- **Linux**: use the `.AppImage` or the `.deb`.

Want to try it with data first? Settings → *Import JSON* → pick
[`samples/sample-data.json`](samples/sample-data.json) (two fictional years of practice).
Create a separate user under Settings → *Users* so the sample stays apart from your own diary.

## Where your data is

macOS: `~/Library/Application Support/jp.chanote.app/` — `chanote.sqlite` and
`attachments/`. Windows: `%APPDATA%\jp.chanote.app\`. Linux: `~/.local/share/jp.chanote.app/`.
Settings → *Save backup* writes a zip of both; *Restore from backup* loads it back.

## Building from source

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT — see [LICENSE](LICENSE).
