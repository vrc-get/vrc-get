# GPUI staged migration track

This repository now contains a staged migration track to move GUI rendering from Tauri/WebView to GPUI without deleting the working Tauri app.

## Scope

- Keep `vrc-get-gui` (Tauri) as the production frontend until feature parity is reached.
- Add `vrc-get-gui-gpui` as an experimental frontend crate in the same workspace.
- Keep `vrc-get-vpm` as the shared business/backend library for both frontends.
- Introduce `vrc-get-gui-runtime` for a shared Tokio runtime bridge pattern.

## Stages

### Stage 1 – Validated ✅

Package management table POC (`app/_main/projects/manage/-package-list-card.tsx` equivalent):
- GPUI table rendering with striped rows and column headers.
- Text input with clear button and live search filtering.
- Dialog lifecycle (title, confirm button, child content).
- Native file dialog integration via `rfd`.
- `TokioBridge` async plumbing (`spawn` / `call` / `shutdown`).

### Stage 2 – In progress

Wire real `vrc-get-vpm` data into a live Projects list screen:
- `backend.rs` — async `load_projects()` using `VccDatabaseConnection`.
- `ProjectsView` — loading state → live data, live search filtering via `cx.observe`.
- `TokioBridge::call()` dispatches to Tokio; result is awaited in GPUI's async context via `cx.spawn`.

### Stage 3 – Planned

Port pages in this order:
1. Setup wizard
2. Settings
3. Log viewer
4. Projects (full, with create/add/remove)
5. Packages (last, hardest)

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

## Async bridge pattern

```rust
// Dispatch heavy async work to Tokio; await result in GPUI.
let rx = self.bridge.call(load_projects()).unwrap();
cx.spawn(async move |this: WeakEntity<View>, cx: &mut AsyncApp| {
    if let Ok(Ok(data)) = rx.await {
        this.update(cx, |view, cx| {
            view.data = data;
            cx.notify();
        }).ok();
    }
}).detach();
```

The pattern works because `tokio::sync::oneshot::Receiver<T>` implements `Future` and can be awaited from within GPUI's executor.
