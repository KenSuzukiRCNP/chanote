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
```

### Axis values usable in `defaults` / `modifiers`

| key | values |
|---|---|
| `tea` | `koicha`, `usucha`, `tsuzuki` (thick then thin), `"-"` (no tea) |
| `tana` | `tana.<id>` from this pack, or `"-"` for carried-in (運び) |
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
