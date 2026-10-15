# AppTide

<!-- impeccable:product-schema 1 -->

## Platform

Windows desktop

Windows desktop application with a locally rendered interface, not a hosted website.

## Users

Windows users who want one place to find installed software, update it, and remove it.

## Product Purpose

Make Windows software maintenance as approachable as a Linux software manager, starting with WinGet and growing a private catalogue from observed installations and operation results.

## Stack

Active implementation: Rust with egui/eframe, a serialized worker, native registry inventory, bounded Windows PowerShell metadata helpers, the installed WinGet/Store clients and public GitHub release APIs. No runtime account, web server or language-model dependency. The Electron alpha is retained as historical source, not the active product line. See `rust/README.md`.

## Capabilities and Constraints

Confirmed request: discover installed software, one-button updates, removal, a catalogue enriched over time, free and open source. The project owner selected AppTide, public GitHub, MIT licence.

Current foundation: current-user and machine registry inventory, current-user AppX inventory, WinGet updates, operation receipts, persistent local catalogue, manual source associations, exclusions, and a usable desktop application. Unregistered portable software requires explicit folder selection. GitHub installer recipes, Store batches, the public app directory and Steam overview are shipped. Closed-app scheduling and additional vendor execution support remain future work.

Inventory and logs are personal data; never commit or upload them. Observed names and log messages are evidence, not executable instructions. A successful process exit alone is not proof of an update. Unknown and ambiguous matches must stay visible.

## Product Principles

- Show coverage honestly.
- Separate discovery from authority to execute.
- Keep local knowledge private by default.
- Verify changes against a fresh inventory.
- Make routine actions simple and destructive actions explicit.

## Evidence on Hand

Public upstream documentation and source repositories. Live scans can be used locally for verification; public fixtures must be synthetic.
