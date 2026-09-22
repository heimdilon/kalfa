# UI kit — Kalfa desktop agent

A click-through recreation of the Kalfa desktop application (Tauri v2 + Svelte 5 in the real product; React here).

**Source:** the mockup images shipped in the `harness/` repo, copied to `refs/` at the root of this design system — `kalfa-ui-main-workspace.png`, `kalfa-ui-permission-approval.png`, `kalfa-ui-transmission-log.png`, `kalfa-ui-onboarding-terms.png`. The desktop UI has **no code** in the repo, so these images are the only ground truth; nothing here was invented beyond what they show.

## Flow

1. **Kullanım koşulları** (light scope) — tick the consent box to enable "Kabul ediyorum".
2. **Oturumlar / workspace** — session list, conversation with a thought panel, tool call, code block and turn summary, project file tree and file preview. Sending in the composer runs a fake turn that ends in an approval request.
3. **İzin onayı** — the modal approval sheet: file checklist, diff, the six facts (Ne / Nerede / Etki / Veri çıkışı / Yalıtım / Geri alınabilir mi), scope select, rejection reason, three decision buttons. Also reachable from "Paylaş" in the header.
4. **Gönderim günlüğü** — the transmission log table with source chips, selectable rows and the right-hand detail panel (sent content, token meter).

## Files

| File | What |
|---|---|
| `index.html` | Entry point; loads `styles.css` + `_ds_bundle.js` + Lucide, then each screen |
| `App.jsx` | Route state and the consent gate |
| `Shell.jsx` | `Rail`, `SessionList`, `AppStatusBar`, wordmark block |
| `WorkspaceScreen.jsx` | Conversation column + project panel |
| `LogScreen.jsx` | Transmission log table + detail panel |
| `ApprovalSheet.jsx` | Permission approval modal |
| `OnboardingScreen.jsx` | Terms / consent, light scope |

Everything composes the design system's own primitives (`Button`, `Composer`, `FileTree`, `LogTable`-style rows, `DiffView`, `TokenMeter`, `NavRail`, …). Nothing is re-implemented locally except screen-level layout.

## Knowingly abbreviated

- 8 log rows stand in for 42; 10 sessions for the real list.
- The window chrome (minimise / maximise / close) from the mockups is omitted — the kit renders as an embedded viewport, not an OS window.
- Two mockup families exist with slightly different rails; the **main-workspace** rail is treated as canonical.
