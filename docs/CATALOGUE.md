# AppTide's software knowledge

## Two separate stores

The bundled database is `rust/data/recipes.json`: versioned JSON with reviewed public product names, exact publishers, update owners, instructions, official sources, review dates, and optional GitHub release repositories. Beta.2 ships ten product/family recipes plus two channel rules. Rust embeds the database in the executable, so it works without downloading a catalogue.

The private catalogue is `%LOCALAPPDATA%\AppTide-Beta\catalog.json`. It accumulates first/last sightings, versions, preferences, release associations, log sightings and verified/unverified operation counts. An unknown app is a **Needs research** entry, not a failure or an invitation to guess an installer. Public recipes and private observations have different authority. No inventory is uploaded and no private observation becomes a public contribution automatically.

Structured records fit this task better than retrieval-augmented generation: identity, version comparison and installer permission need reproducible rules. The database neither needs an AI model nor treats text from logs as commands.

## Recognition and update routes

- Registry products require an anchored product-name pattern and an exact publisher. Multiple matching recipes yield no match. Private forks with different identities are left alone.
- Steam game ownership comes from a numeric Steam App registration. Steam manages updates; AppTide does not infer the latest game build. See [Steam's update documentation](https://help.steampowered.com/en/faqs/view/71AB-698D-57EB-178C).
- AppX records require Windows `SignatureKind=Store` before being assigned to Microsoft Store. A name beginning with Microsoft is insufficient, and a Developer signature is not a Store signature. See [Microsoft's signature kinds](https://learn.microsoft.com/en-us/uwp/api/windows.applicationmodel.packagesignaturekind) and [Store updates](https://support.microsoft.com/en-us/windows/apps/turn-on-automatic-app-updates).
- Other reviewed routes include Creative Cloud, NVIDIA App, Battle.net, browser updaters and official vendor downloads. Each recipe includes its source. These routes provide instructions, not an independently verified update state.
- Identical current-user registry registrations shared across 32/64-bit views are collapsed. Different keys, versions, publishers, locations and protection flags remain distinct. This is not universal product deduplication.

## GitHub release watches

Select an app and save `owner/repository` in its details, or use its bundled association. Checks use the public [latest stable release API](https://docs.github.com/en/rest/releases/releases#get-the-latest-release), without credentials, inventory or local paths. Successful checks are cached for six hours during scans; explicit refresh has a one-minute cooldown. Each pass checks at most 20 repositories and stops on a network/rate-limit error. Response size and process duration are bounded.

Numeric tags can be compared with numeric installed versions (including leading v/V and `_32`/`_64` packaging suffixes). Other formats require review. A newer tag is not proof that an asset is compatible. AppTide opens the releases page; it does not download or run arbitrary assets. Changing an observed publisher clears release associations and automatic-update consent. Cached comparisons are recomputed when the installed version changes.

Initial public release sources include [ExifToolGUI](https://github.com/FrankBijnen/ExifToolGui/releases) and [Meetily Community](https://github.com/Zackriya-Solutions/meetily/releases). An edition or private fork must not be silently mapped to another product.

## Private learning

Every scan preserves version history and adds observations. A unique native registration that becomes WinGet-correlated retains its history. Local `learning` records summarize retained version sightings and operation outcomes. Failed/unverified operations block automatic retries; successful manual verification can unblock them. Publisher changes revoke automatic consent. None of these observations bypasses WinGet identity checks.

Use Installed apps or My catalogue filters to see **Beyond WinGet**, **Needs research**, or **Release watches**. The primary action opens the official instructions or releases for known custom routes. Direct automatic updates remain restricted to WinGet.

## Contributing a bundled recipe

Start with public documentation and synthetic metadata fixtures. Include a narrow identity rule, an official source, a review date and a plain-language update route. Never copy someone's inventory, local paths, logs, credentials or private product names into a pull request. Review false positives, architecture, editions and release channels. Detection guidance is not an executable update recipe.

Automatic custom installers remain a later milestone: immutable assets, integrity/signature policy, architecture/scope selection and disposable-Windows installation/upgrade/removal checks must come first. Retain old releases and changelog entries when the database evolves.
