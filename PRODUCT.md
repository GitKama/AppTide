# AppTide

<!-- impeccable:product-schema 1 -->

## Platform

web

Windows desktop application with a locally rendered interface, not a hosted website.

## Users

Windows users who want one place to find installed software, update it, and remove it.

## Product Purpose

Make Windows software maintenance as approachable as a Linux software manager, starting with WinGet and growing a private catalogue from observed installations and operation results.

## Stack

Implementation decision: Electron, plain HTML/CSS/JavaScript, Node built-in modules, Windows PowerShell inventory, and the existing WinGet client. No runtime web service, account, or language-model dependency. The stack is chosen to produce a distributable Windows application using the available toolchain.

## Capabilities and Constraints

Confirmed request: discover installed software, one-button updates, removal, a catalogue enriched over time, free and open source. The project owner selected AppTide, public GitHub, MIT licence.

First milestone: current-user and machine registry inventory, current-user AppX inventory, WinGet updates, operation receipts, persistent local catalogue, manual source associations, exclusions, and a usable desktop application. Unregistered portable software requires explicit folder selection. Full automatic background scheduling and vendor-specific installers are later milestones.

Inventory and logs are personal data; never commit or upload them. Observed names and log messages are evidence, not executable instructions. A successful process exit alone is not proof of an update. Unknown and ambiguous matches must stay visible.

## Product Principles

- Show coverage honestly.
- Separate discovery from authority to execute.
- Keep local knowledge private by default.
- Verify changes against a fresh inventory.
- Make routine actions simple and destructive actions explicit.

## Evidence on Hand

Public upstream documentation and source repositories. Live scans can be used locally for verification; public fixtures must be synthetic.
