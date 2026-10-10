# Rust beta implementation decision

Researched 2026-10-10. User direction: native Rust, a custom immediate-mode UI, and a beta milestone.

## UI choice

[egui](https://github.com/emilk/egui) is a Rust immediate-mode GUI library. Its official [eframe integration](https://github.com/emilk/egui/blob/main/crates/eframe/README.md) supplies desktop windows, input, rendering, and platform integration. AppTide owns the layout, app-row painting, palette, typography, state, and interactions. This is a custom application UI on an established renderer, not a new GUI toolkit.

Pinned eframe 0.36.2, with its current `App::ui` API. Its [crate metadata](https://crates.io/crates/eframe/0.36.2) requires Rust 1.95. The [App API](https://docs.rs/eframe/0.36.2/eframe/trait.App.html) supports requesting a repaint when worker results arrive. The UI does not deliberately redraw continuously when idle. `ScrollArea::show_rows` limits painting to the visible list rows. AccessKit is enabled; screen-reader behavior still needs user-level validation.

The first beta uses eframe's Glow/OpenGL backend to keep this initial distribution and dependency surface smaller. GPU-driver compatibility must be tested across Windows machines; wgpu/Direct3D is a future alternative if measurements or compatibility justify it. This choice is not a claim that OpenGL is faster than Direct3D.

Tauri retains a web UI and therefore does not meet this request for an immediate-mode interface. Native Windows UI frameworks provide stronger Windows control conventions but use a different UI model. egui fits this requested experiment directly.

Rust can remove the bundled Chromium/Node runtime and offers native compilation and explicit ownership. It does not automatically make a UI attractive or make network/installer operations faster. Presentation comes from design; responsiveness comes from bounded work, list virtualization, and keeping blocking operations away from the UI thread. Performance claims should use packaged-build measurements.

## Windows integration

Registry discovery is native Rust through winreg. AppX discovery currently uses the installed Windows `Get-AppxPackage` cmdlet, through a fixed read-only script with no interpolated app data. It does not change PowerShell execution policy. A future port can use the Windows Runtime [PackageManager](https://microsoft.github.io/windows-docs-rs/doc/windows/Management/Deployment/struct.PackageManager.html). That API inventories AppX; it must not be confused with WinGet's deployment API.

WinGet execution remains an argument-array child process with exact package identity and target version. CLI table parsing retains fail-closed handling of truncated/unsupported rows. It does not solve source coverage or structured WinGet integration yet. stdout and stderr are drained concurrently and bounded; scans time out, while installer operations are not forcibly terminated.

## Layout contract

The established AppTide slate/teal identity is preserved. The native beta uses a fixed navigation column, a virtualized centre list, and a persistent right pane. Update/uninstall controls live above the details scroll area. Scrolling either list cannot move those controls out of view. Minimum logical window size is 1040 × 650 for this three-column beta.

## Migration and release boundary

The beta opens a separate local data folder with an exclusive process lock. Alpha import is available only into an empty beta catalogue and resets automatic execution consent. Schema extensions such as saved watches are preserved. The old alpha stays runnable.

Beta.1 is an initial native implementation, not a declaration of installer reliability. Unattended updates, portable discovery, and release-watch editing still need porting. Real third-party update/removal behavior requires a disposable Windows test environment; synthetic success does not prove live installer behavior.
