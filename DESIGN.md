# AppTide interface

Operate mode: a quiet, practical Windows maintenance desk. App rows are the primary surface; a persistent right pane explains each source and its evidence. Native Segoe UI typography, slate navigation, white list surfaces, restrained teal actions, and distinct amber/red attention states.

Palette: ink #24354a, secondary text #596c80, background #f7f9fb, sidebar #eaf0f5, action #076d72, rule #dfe6ed. Type scale 11/12/13/14/19/28 px. Standard controls and visible keyboard focus take precedence over brand expression.

At 1320px: 205px navigation, flexible application list, 288px details. At narrow desktop widths the details move below the list; the native window minimum is 820px. No decorative animation. All dynamic strings are escaped. Demo data is explicitly identified at the top of the window.

The details pane stays pinned within the viewport beside long application lists and scrolls independently when its contents exceed the window height. Selecting another app resets the pane to its heading and action controls. On narrow windows the pane stacks below the list, and selecting an app brings it into view.

The memorable element is the catalogue's evidence trail: version observations and log sightings beside each app. Log observations never imply a verified update. Source coverage remains visible even when no updates are available.

Design decision: build directly from the operating task and review rendered desktop states. No user-specific aesthetic preference is inferred or saved.
