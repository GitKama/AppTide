# AppTide's software knowledge

## Public sources and private observations

The bundled database is `rust/data/recipes.json`: versioned JSON with reviewed public product names, exact publishers, update owners, instructions, official sources, review dates, and optional GitHub release repositories. Beta.2 introduced ten product/family recipes and two channel rules; 0.2.1 adds 7-Zip, VLC, KeePass and Notepad++ for fourteen product/family recipes plus the channel rules. Rust embeds the database in the executable, so it works without downloading a catalogue.

Alongside the fourteen reviewed recipes, `rust/data/community.json` embeds 19,754 metadata-only package records from WinGet and Scoop, including 2,755 distinct GitHub projects. This searchable directory supports corroborated source associations, not arbitrary installer execution. See [provenance, licences and matching rules](PUBLIC-CATALOGUE.md).

The private catalogue is `%LOCALAPPDATA%\AppTide-Beta\catalog.json`. It accumulates first/last sightings, versions, preferences, release associations, log sightings and verified/unverified operation counts. An unknown app is a **Needs research** entry, not a failure or an invitation to guess an installer. Public recipes and private observations have different authority. No inventory is uploaded and no private observation becomes a public contribution automatically.

Structured records fit this task better than retrieval-augmented generation: identity, version comparison and installer permission need reproducible rules. The database neither needs an AI model nor treats text from logs as commands.

## Maintain your own catalogue

Choose **My catalogue > Add custom app**, enter a name, optional manually maintained version, official HTTPS page, optional GitHub project URL and update notes. The entry is explicitly marked manually tracked; AppTide does not claim it was detected on disk. Rename it or change its version and source using **Edit my catalogue entry**. **Stop tracking** retains its history without changing installed software.

The same editor can attach sources and notes to detected apps, while their detected name/version remain inventory-owned. A GitHub URL or `owner/repository` is accepted. Clearing the watch disables its bundled release association too. HTTPS source links open in the browser; saved notes are never commands. No local entries are uploaded or silently added to the public database.

## Recognition and update routes

- Registry products require an anchored product-name pattern and an exact publisher. Multiple matching recipes yield no match. Private forks with different identities are left alone.
- Steam ownership comes from numeric Steam registrations and local library manifests. The separate Steam library shows local build IDs and download state. Steam manages updates; AppTide does not infer the latest game build. See [Steam's update documentation](https://help.steampowered.com/en/faqs/view/71AB-698D-57EB-178C).
- AppX records require Windows `SignatureKind=Store` before being assigned to Microsoft Store. A name beginning with Microsoft is insufficient, and a Developer signature is not a Store signature. See [Microsoft's signature kinds](https://learn.microsoft.com/en-us/uwp/api/windows.applicationmodel.packagesignaturekind) and [Store updates](https://support.microsoft.com/en-us/windows/apps/turn-on-automatic-app-updates).
- Other reviewed routes include Creative Cloud, NVIDIA App, Battle.net, browser updaters and official vendor downloads. Each recipe includes its source. These routes provide instructions, not an independently verified update state.
- Identical current-user registry registrations shared across 32/64-bit views are collapsed. Different keys, versions, publishers, locations and protection flags remain distinct. This is not universal product deduplication.

## GitHub updates

Projects are detected from unambiguous HTTPS GitHub URLs in installed metadata, reviewed name/publisher recipes, or corroborated public directory matches. Explicit user choices take precedence; clearing a project disables detection for that entry. Conflicting metadata stays unresolved.

Public stable releases are checked without credentials or uploading inventory. Successful checks are cached for one hour; explicit refresh is immediate. Each scan checks at most 20 repositories; failures are recorded without abandoning the remaining repositories. Old watch-only cache records are refreshed to obtain release assets.

Compatible installers now participate in individual and bulk updates. Reviewed recipes add unattended support, with separate per-app consent. The full selection, integrity, scope and verification policy is documented in [GitHub updates](GITHUB-UPDATES.md). Unsupported assets remain release links.

## Private learning

Every scan preserves version history and adds observations. A unique native registration that becomes WinGet-correlated retains its history. Local `learning` records summarize retained version sightings and operation outcomes. Failed/unverified operations block automatic retries; successful manual verification can unblock them. Publisher changes revoke automatic consent. None of these observations bypasses provider identity or installer-integrity checks.

Use Installed apps or My catalogue filters to see **Beyond WinGet**, **Needs research**, or **Release watches**. The primary action opens the official instructions or releases for known custom routes. Direct automatic updates support WinGet and the reviewed GitHub user-installation recipes.

## Contributing a bundled recipe

Start with public documentation and synthetic metadata fixtures. Include a narrow identity rule, an official source, a review date and a plain-language update route. Never copy someone's inventory, local paths, logs, credentials or private product names into a pull request. Review false positives, architecture, editions and release channels. Detection guidance is not an executable update recipe.

New automatic installer recipes require a reviewed upstream build definition, narrow identity and scope rules, release asset integrity, and installation testing. Portable archive replacement remains unsupported. Retain old releases and changelog entries when the database evolves.
