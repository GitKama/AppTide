# AppTide interface

Operate mode: a quiet, practical Windows maintenance desk. App rows are the primary surface; a persistent right pane explains each source and its evidence. Native Segoe UI typography, slate navigation, neutral list surfaces, restrained teal actions, and distinct amber/red attention states.

The Rust beta opens in dark mode. The canvas is #161d25, the sidebar is #1d2833, body text is #e6edf3, secondary text is #b0c0d0, and teal actions use #76dad2. Settings → Appearance saves a light or dark choice locally. Light mode retains the original slate and teal palette. Type scale 11/12/13/14/19/28 px. Standard controls and visible keyboard focus take precedence over brand expression.

At 1320px: 205px navigation, flexible application list, 288px details. At narrow desktop widths the details move below the list; the native window minimum is 820px. No decorative animation. All dynamic strings are escaped. Demo data is explicitly identified at the top of the window.

The details pane stays pinned within the viewport beside long application lists and scrolls independently when its contents exceed the window height. Selecting another app resets the pane to its heading and action controls. On narrow windows the pane stacks below the list, and selecting an app brings it into view.

The memorable element is the catalogue's evidence trail: version observations and log sightings beside each app. Log observations never imply a verified update. Source coverage remains visible even when no updates are available.

Design decision: build directly from the operating task and review rendered desktop states. No user-specific aesthetic preference is inferred or saved.

Rust beta: preserve this identity using custom egui rows and panels. The three-column layout has a 1040×650 logical minimum window size. Only the centre list and lower details scroll; the right-side update/uninstall controls stay fixed. Long names in the fixed header truncate with full-text hover hints. Use Windows' installed Segoe UI without redistributing it. Render visible rows and repaint on input/worker events.
