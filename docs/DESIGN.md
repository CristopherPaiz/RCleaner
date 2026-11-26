# Interface design

English · [Español](DESIGN.es.md)

References studied: **CleanMyMac** (MacPaw, Red Dot / iF awards for its UI),
**DaisyDisk** (radial disk map) and **Ashampoo WinOptimizer** (dense, clear
result list on Windows). One idea was taken from each, not its look:

| Reference | Idea adopted |
|---|---|
| CleanMyMac | One tool per screen, each with **its own color**, a large glyph and **a single round button** at the bottom center, always in the same place (Scan → Stop → Clean → Done). Short side menu of icons. |
| DaisyDisk | **Radial** map by folder, sorted by size, with tiny items grouped; center = current folder (click to go up); list beside it; a "tray" to choose what to delete without deleting right away. |
| WinOptimizer | Results as **rows per category** (icon, name, item count, size, expand) instead of cards. Big total figure at the top. |

## Rules

1. **No bordered cards everywhere.** Separate with space and thin lines
   (`--line`). Only dialogs and Settings groups get a surface.
2. **One main action per screen**, always in the `Dock` (round button).
   Secondary actions are subtle buttons (`.btn.quiet`).
3. **Color per section** via `[data-section]` → variables `--a` / `--b`:
   Overview blue, Cleanup green, Space violet, Duplicates orange, Settings gray.
   The rail, glyph, button, checkboxes and background glow inherit it.
4. **Big figures** with the unit in small type (`splitBytes`), tabular numbers.
5. **Risk as text with a colored dot**, not pills.
6. Native typography: *Segoe UI Variable Display* (titles) and *Text* (the rest).
7. Light/dark following the system (or the user's choice). Short animations
   (≤ 350 ms), turned off with `prefers-reduced-motion`.

## Pieces

| Component | Use |
|---|---|
| `Glyph` | Tile with the section gradient + icon; spinning ring with `busy`. |
| `ActionButton` | Main round button (`go` / `stop`). |
| `Dock` | Bottom bar: left (secondary), center (button), right (summary). |
| `Modal` | Dialog with a blurred backdrop; Escape/click outside closes it. |
| `Sunburst` | Radial map (`lib/sunburst.ts` computes arcs and colors). |
| `ResultList` | Collapsible rows per category with their findings. |

Tokens in `app/src/app.css`.
