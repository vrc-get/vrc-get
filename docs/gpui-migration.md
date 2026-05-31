# GPUI staged migration track

This repository now contains a staged migration track to move GUI rendering from Tauri/WebView to GPUI without deleting the working Tauri app.

## Scope

- Keep `vrc-get-gui` (Tauri) as the production frontend until feature parity is reached.
- Add `vrc-get-gui-gpui` as an experimental frontend crate in the same workspace.
- Keep `vrc-get-vpm` as the shared business/backend library for both frontends.
- Introduce `vrc-get-gui-runtime` for a shared Tokio runtime bridge pattern.

## Stages

1. Validate with the make-or-break screen first: package management table (`app/_main/projects/manage/-package-list-card.tsx`), including text input and dialog interaction.
2. Keep backend command layer in Rust/Tokio and migrate frontend incrementally.
3. Port pages in this order:
   - Setup wizard
   - Settings
   - Log viewer
   - Projects
   - Packages (last, hardest)

## i18n migration

- Script added: `vrc-get-gui/scripts/i18next-to-rust-i18n.mjs`
- Converts i18next dotted-key JSON5 format to nested rust-i18n YAML.
- Run with:
  - `npm run i18n:to-rust`
  - `npm run i18n:to-rust -- locales/ja.json5 locales/ja.yml`

## Native file dialog policy

- GPUI migration path uses `rfd` for native file/folder dialogs on Windows/macOS/Linux.

## GPUI version pinning

- GPUI is pinned to Zed commit `69e2130295c2649963eb639fc70b4f2ee8ea1624` in workspace patch configuration.
- Update only by intentional SHA bumps.

## Linux GPU note

- GPUI with Vulkan generally behaves better than WebKit for open-source NVIDIA users.
- Nouveau may fall back to llvmpipe (software rendering).
- Mesa + AMD/Intel Vulkan is the expected reliable path.
