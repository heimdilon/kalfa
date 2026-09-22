# Kalfa Design System

Kalfa is a **Turkish-language, open-source agent harness for the EVREN LLM Gateway** — a Rust desktop application that reads, writes, searches and runs commands on your machine, always behind an explicit approval. Tools are extensible through isolated WebAssembly plugins.

The name comes from the Ahi guild ladder — *çırak → kalfa → usta* (apprentice → journeyman → master). A *kalfa* is a capable craftsman who works under the master's direction, and that is literally the product's permission model: skilled, but you have the last word. Every visual decision in this system serves that idea — **competent, quiet, and never acting without consent.**

## Products / surfaces

| Surface | State in source | Covered here |
|---|---|---|
| **Kalfa desktop app** (Tauri v2 + Svelte 5, Windows + Linux) | UI **not yet implemented** in code; design exists as mockups | `ui_kits/desktop/` |
| `kalfa-core` / `kalfa-evren` / `kalfa-tools` (Rust crates) | Implemented | Vocabulary, states and copy sourced from here |
| Plugin SDK (WASM Component Model / WIT) | Contract stage | Not a UI surface |

There is **no marketing website, docs site or mobile app** in the sources, so this system defines one product surface only.

## Sources this system was built from

- **Attached local codebase:** `harness/` — the Kalfa Rust workspace (`crates/kalfa-core`, `crates/kalfa-evren`, `crates/kalfa-tools`, `docs/GELISTIRME_DURUMU.md`, `docs/superpowers/`, `research/evren-probe/` API fixtures).
- **UI mockup images shipped in that repo** (`harness/Screenshot 2026-09-21 *.png`, `harness/tasks.png`) — copied into `refs/` here. These are the **only** visual source; the desktop UI has no code yet. Every colour, metric and radius in `tokens/` was measured off them and cross-checked against the crates' state vocabulary.
- License / attribution: Apache-2.0, © Orkun Koray Güngör. Kalfa is independent and unaffiliated with SSB or the EVREN platform.
- No Figma file, no font binaries and **no logo asset** were provided.

> **Caveat, read this first:** because the desktop UI exists only as mockups, this design system is a *faithful systematisation of those mockups*, not a recreation of shipped code. Where a mockup was ambiguous the system chose the reading most consistent with the rest of the app, and says so below.

---

## Content fundamentals

**Language is Turkish. Always.** Not "localised English" — the product is written in Turkish first, and every label, empty state and error in this system is Turkish. Do not mix English UI words into a Kalfa interface (no "Settings", no "Send"); technical identifiers (`reasoning`, `glm-5.3`, `req_8f3a`, file paths) stay verbatim because they are data, not prose.

**Casing: sentence case everywhere.** "İletim kaydı", not "İletim Kaydı". The only uppercase is the letterspaced 11px section label in the rail ("ÇALIŞMA ALANI"). The wordmark is lowercase: **kalfa**.

**Person.** The repo's developer-facing prose uses informal *sen* ("senin onayınla", "Eklentini derleyip dağıtırken"). The **product UI uses formal *siz*** — "Bir şey sorun veya bir görev tanımlayın…", "Göndermeden önce göster", "Reddetme nedeninizi buraya yazabilirsiniz…". Keep that split: docs and README are *sen*, interface is *siz*. Kalfa refers to itself in the first person sparingly and never anthropomorphises ("İnceledim" is fine; "Senin için sabırsızlanıyorum" is not).

**Verb-first imperatives for actions, past-tense results for records.** Buttons: "Geri al", "Tümünü onayla", "Kabul etmiyorum". Tool cards: "Dosya okundu", "Komut çalıştırıldı" — never "Dosyayı oku". A finished thing is reported as finished; an offer is phrased as a request.

**Certainty is a house style.** Kalfa distinguishes what it knows from what it observed. Tokens are exact and shown plainly ("2.341 token"); credit cost is an *observation* and is never phrased as a settled price. Irreversibility is always stated, never implied: "Kabuk etkileri geri alınamaz." When Kalfa truncates, it says so (`···`). When state is unknown after a crash, the status is literally "Bilinmiyor" — it does not guess and does not silently re-run.

**No exclamation marks. No emoji. No congratulation.** Completing a task produces "Tamamlandı" and a summary, not "Harika! 🎉". No marketing adjectives, no "seamless", no "powerful".

**Numbers follow Turkish convention:** thousands separated with a dot (2.341), decimals with a comma (0,42), 24-hour time (14:02:11), dates as "24 Nis 2025, 10:14".

**Microcopy length.** Row labels ≤ 3 words. Button labels ≤ 3 words. Hints one clause, lowercase-start, no terminal period ("kurtarma ve geri alma çalışmaz"). Placeholders end in an ellipsis ("Oturumlarda ara…").

---

## Visual foundations

**Colour.** Two neutral families and exactly one accent. The product default is **dark warm charcoal** — `--warm-850` app, `--warm-900` rail and panels, `--warm-750` cards. Warmth is deliberate: these greys are green-blue-neutralised, not the blue-black of a generic IDE theme. A **warm paper** family (`--paper-*`) exists for the light scope — onboarding, terms, consent documents — applied by adding `.kalfa-light` to a container, never by a global theme switch. The single accent is **copper** (`--copper-400 #b8754f` fill, `--copper-200 #f3ab78` text), a metalworker's colour chosen for the *ahi* craft reference. Copper means *action or your attention*: primary buttons, selected rows, the composer border, folder glyphs. It is never decorative. Semantic colours are muted, not neon: green ok, amber caution, red danger, blue info — always as a ~15% tinted fill with a ~45% border of the same hue, never a solid saturated block. **No gradient appears anywhere in this product.** No purple, no blue-violet, no duotone.

**Type.** Source Sans 3 for all UI, JetBrains Mono for all data, Source Serif 4 for documents only. The rule that matters: **mono is for anything the machine is certain about** — ids, paths, line ranges, timestamps, token counts, model names, shell commands. Sans is for anything addressed to a person. Serif appears only in the light document scope (terms, consent). Scale is tight and small: 14px body, 13px secondary, 12px meta, 11px labels, 17px panel titles, 28px display, 38px wordmark (Light, −0.02em). Line-height 1.55 body, 1.7 for serif documents, 1.15–1.35 for headings.

**Spacing and layout.** A 2/4/6/8/10/12/14/16/20/24/32/40/48/64 scale; panel rhythm is 6–8–12, block rhythm 16–20. The shell is a **fixed four-column desktop layout**, not a responsive grid: 213px rail · 295px session list · fluid conversation · 275px inspector, with a 44px title bar and a 46px status bar. Those bars are permanently fixed — the status bar's isolation/model/token readout is a product promise and must never scroll away. Rows are 34px; controls 34px (42px for the large consent button); minimum hit target 32px.

**Backgrounds.** Flat colour. No photography, no illustration, no pattern, no texture, no noise, no full-bleed imagery anywhere in the mockups. Depth is expressed as a **surface value ladder** (rail darker → app → sunken → card → raised lighter) plus a 1px hairline. This is the system's single most characteristic trait: *Kalfa separates things by value and hairline, not by shadow.*

**Shadows.** In dark mode, effectively none. `--shadow-card` (1px, 35% black) is the most a card ever gets, and most get nothing. Real shadows appear in exactly two places: floating overlays (dropdown menus, the approval sheet) use `--shadow-overlay`, and the **light scope** uses a soft warm shadow (`--shadow-paper`) because value contrast is unavailable on paper. The focus ring is a 3px copper halo at 28% (`--shadow-focus`), plus a 2px outline with 2px offset.

**Borders and radii.** 1px hairline is the default edge (`--border-subtle` between panels, `--border-default` on controls). **1.5px copper** marks the two things that are live: the composer, and anything awaiting your decision. Radii: 4 gutter marks · 6 chips and source tags · 8 buttons, inputs, list rows · 10 cards and dropdowns · 12 message bubbles and tool-call cards · 16 the approval sheet · pill for scrollbars and counters. Cards are: 10–12px radius, hairline border, card-value fill, **no shadow**.

**Selection is one language across the whole app.** A selected nav item, session row, log row and file-tree row all get the same treatment: copper tint fill (`--surface-selected`) plus a **3px copper left edge** (`--shadow-accent-edge`) plus a weight bump to semibold. Nothing else in the product uses a left accent edge, so the motif reads unambiguously as "this one".

**Hover and press.** Hover *lightens* — a transparent control picks up `--surface-raised`; a filled copper button goes to `--copper-300`. Press *darkens* to `--copper-500` and nudges the element **0.5px down**. No scale, no spring, no shadow lift, no ripple. Disabled is 55% opacity with muted text and a subtle border — never a greyed-out fill.

**Motion.** Short and flat. 80ms for colour, 120ms for controls, 180ms for panels and disclosure, 260ms for the approval sheet, on `cubic-bezier(.2,0,.2,1)`. Streaming text batches on a 16ms window. Transitions are fades and small translations only; **nothing bounces, nothing scales, nothing spins** except the single loader glyph. A serious product about consent does not have playful motion.

**Transparency and blur.** Used sparingly and only functionally. `color-mix` alphas build the semantic tint fills; the modal scrim is `rgba(8,10,12,.62)` with a light `blur(2px)` — the only blur in the product. No frosted-glass panels, no translucent sidebars.

**Imagery.** There is none, by design. Kalfa has no photography, no avatars and no illustration. Identity is a **tinted glyph tile** (`IconTile`): the user is a `user` glyph, Kalfa is a copper tile, a tool is its own glyph. If a design ever needs an image, that is a signal the design has left the product's vocabulary.

---

## Iconography

**Lucide, at 1.5–2px stroke, outline only.** The Kalfa repo contains no icon assets, no icon font and no SVG sprite (the UI is unimplemented), and the mockups' glyphs match the Lucide set. This is therefore a **flagged substitution**: `Icon` renders Lucide from CDN —

```html
<script src="https://unpkg.com/lucide@0.460.0/dist/umd/lucide.js"></script>
```

— and if Kalfa later ships its own set, swapping it means editing `components/core/Icon.jsx` alone.

- **Never hand-draw an SVG path** in a Kalfa design. Every glyph goes through `<Icon name="…" />`.
- **Sizes:** 14 inline with 13px text · 16 in buttons and list rows · 18 in rail nav · 20 in panel headers. Nothing larger; there are no hero icons.
- **Colour** always comes from a text token (`currentColor` by default). Copper glyphs mark folders, the agent, and active nav. Semantic glyph colour only for a real state.
- **The working vocabulary:** `shield-check` / `shield-alert` (isolation, permission), `messages-square` (sessions), `scroll-text` (transmission log), `folder-tree` / `folder` / `file-text` (workspace), `terminal` (shell), `box` (model), `wrench` (tools), `plug` (plugins), `rotate-ccw` (undo), `check` / `x` (approve / reject), `paperclip`, `send`, `search`, `settings`, `activity` (tokens), `loader-circle` (the only animated glyph).
- **No emoji, ever** — not in UI, not in copy, not in commit-style status. **No unicode pictographs as icons.** Two unicode characters *are* sanctioned as typographic marks: `···` for truncation and `·` as the meta separator.
- **No logo asset exists in the sources.** Wherever a mark would go, set the name in type: lowercase `kalfa`, Source Sans 3 Light 38px, −0.02em. The copper "k" tile in `guidelines/wordmark.card.html` is an explicitly-labelled app-icon stand-in, not a brand mark. Do not draw, reconstruct or invent one.

---

## Components

Grouped by concern under `components/`. Every one is a self-contained `.jsx` styled only with the CSS custom properties, with a sibling `.d.ts` and `.prompt.md`.

**core/** — `Button`, `IconButton`, `Icon`, `IconTile`, `Badge`, `Card`, `SourceTag`
**forms/** — `Input`, `Textarea`, `Select`, `Checkbox`, `Switch`
**chat/** — `Message`, `Composer`, `ThoughtPanel`, `ToolCallCard`, `TurnSummary`
**data/** — `CodeBlock`, `DiffView`, `FileTree`, `LogTable`, `MetaList`, `TokenMeter`, `Pagination`
**nav/** — `NavRail` (+ `NavSection`), `NavItem`, `SessionRow`, `PanelHeader`, `StatusBar` (+ `StatusItem`)

### Intentional additions

The source defines no component library (the UI is unbuilt), so the inventory was derived from what the mockups and the crates' state vocabulary actually show. Three entries are systematisations rather than literal mockup elements:

- **`Icon`** — a wrapper so the glyph set is swappable in one file.
- **`IconTile`** — the mockups show tinted glyph squares as author identity; this names the pattern.
- **`SourceTag`** — the transmission-log provenance chip, promoted to a primitive because the same chip appears in the composer, the approval sheet and the log detail.

Deliberately **not** built, because nothing in the source shows them: Toast, Tooltip, Avatar, Tabs, Breadcrumb, Accordion, DatePicker.

---

## Index

| Path | What |
|---|---|
| `styles.css` | Global entry — link this one file. `@import` list only. |
| `tokens/` | `colors`, `typography`, `fonts`, `spacing`, `radius`, `elevation`, `motion`, `base` |
| `components/{core,forms,chat,data,nav}/` | Primitives + `.d.ts` + `.prompt.md` + one `@dsCard` HTML each |
| `guidelines/*.card.html` | 20 foundation specimen cards (Colors, Type, Spacing, Surfaces, Brand) |
| `ui_kits/desktop/` | Click-through recreation of the Kalfa desktop app |
| `refs/` | The original mockup screenshots this system was measured from |
| `thumbnail.html` | Homepage tile |
| `SKILL.md` | Agent-skill entry point |

## Known gaps

- **Fonts are substitutes.** The repo ships no binaries. Source Sans 3 / Source Serif 4 / JetBrains Mono were matched to the mockups and loaded from Google Fonts. Send real files and `tokens/fonts.css` is the only file that changes.
- **Icons are substitutes** (Lucide) — see Iconography.
- **No logo** — see Iconography.
- The desktop UI is unimplemented, so nothing here could be verified against shipped code.
