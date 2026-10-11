# School packs

A school pack is one YAML file describing a tradition's procedures (点前), shelves (棚),
hearth arrangements and utensil categories. The bundled pack is
[`packs/urasenke.yaml`](../packs/urasenke.yaml) — read it alongside this page; it is the
best example.

The pack is loaded when the app starts. Users can still rename, hide and add entries in the
list editor; the pack only provides the starting vocabulary and sensible defaults.

## Format

```yaml
id: omotesenke                 # short ascii id, also used as prefix for origin tracking
version: "2026.10.1"           # bump to make the app re-import the pack
name: { ja: 表千家, romaji: Omotesenke, en: Omotesenke school }

categories:                    # groups in the temae picker; `parent` nests them
  - { id: hira,     ja: 平点前, romaji: hiradate, en: Basic procedures }
  - { id: konarai,  ja: 小習,   romaji: konarai }
  - { id: konarai.a, ja: 前段, parent: konarai }

temae:
  - id: hiradate               # stable id — never rename, archive instead
    ja: 平点前                  # required; the name shown to users
    romaji: hiradate           # optional
    en: Basic tea procedure    # optional; shown on hover
    category: hira
    defaults: { tana: "-" }    # values prefilled when the temae is chosen

  - id: hiradate.koicha        # an alias: a familiar name that expands to another temae
    ja: 濃茶点前
    category: hira
    alias_of: hiradate
    modifiers: { tea: koicha }

tana:
  - { id: daisu,      ja: 台子, romaji: daisu }
  - { id: daisu.shin, ja: 真台子, parent: daisu }

hearth:
  - { id: ro,   ja: 炉,   romaji: ro }
  - { id: furo, ja: 風炉, romaji: furo }

tool_categories:
  - { id: chawan, ja: 茶碗, romaji: chawan, en: Tea bowl }

# kinds of event the user can pick when a record is a chaji or chakai (optional)
event_kinds:
  - { id: shogo,     form: chaji,  ja: 正午の茶事, romaji: shōgo no chaji, en: Midday tea gathering }
  - { id: hatsugama, form: chakai, ja: 初釜,       romaji: hatsugama }
```

### Ids in the app

Inside the app, ids are scoped by pack: `hiradate` in `omotesenke.yaml` becomes
`temae.omotesenke.hiradate`, categories `temae-cat.omotesenke.hira`, shelves
`tana.omotesenke.daisu`, event kinds `event.omotesenke.shogo`. Two schools can therefore
both have a `karamono` without colliding. Pack authors never write the prefix; the loader
adds it.

### Axis values usable in `defaults` / `modifiers`

| key | values |
|---|---|
| `tea` | `koicha`, `usucha`, `tsuzuki` (thick then thin), `"-"` (no tea) |
| `tana` | a shelf id from this pack (`daisu.shin`; the older `tana.daisu.shin` spelling is also accepted), or `"-"` for carried-in (運び) |
| `hearth` | `ro`, `furo`, `binkake`, `nakaoki`, `tsurigama`, `dairo`, `sukigi`, `other` |
| `katte` | `jun`, `gyaku` |
| `room` | `koma`, `yojohan`, `hiroma` |
| `kinin` | `"-"`, `kinindate`, `kiyotsugu` |
| `tsukimono` | list of add-ons (used for kagetsu variants) |

Defaults are suggestions — the user can always change them on the record.

## Guidelines

- Keep ids ascii, lowercase, stable. They never appear in the UI.
- Prefer aliases over near-duplicate procedures: "濃茶付花月" is `kagetsu` with modifiers,
  not a separate procedure, so statistics can count all kagetsu together and still show the
  breakdown.
- Categories should follow how the school itself groups and teaches procedures.
- `en` is a short gloss, not a translation of the name.

## Contributing a pack

1. Copy `packs/urasenke.yaml` to `packs/<school>.yaml` and edit.
2. Run `cargo test -p chanote-core` — the pack is validated (aliases must point to existing
   entries, categories must exist).
3. Open a pull request, or simply send the file in an issue if you are not comfortable with
   git. Please say which lineage or textbook the list follows.

Loading a pack other than Urasenke from the app itself is on the roadmap; until then a pack
is bundled at build time.

## Getting a missing procedure into the bundled pack

Add it in the app (Lists → Temae → *Add temae*), then press **YAML** on the row: the
snippet is copied to the clipboard in pack format. Paste it into an issue or a pull
request against `packs/<school>.yaml`. Rows you edit in the app are kept even when the
pack is updated later (they are marked *edited*; *Reset to pack* restores the pack's values).
