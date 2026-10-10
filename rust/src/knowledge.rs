//! Reviewed public recipes are separate from private observations and execution authority.
use crate::catalog::{App, Catalog};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Deserialize)]
pub struct Database {
    pub schema: u32,
    pub revision: String,
    pub recipes: Vec<Recipe>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub id: String,
    pub name_pattern: String,
    pub publishers: Vec<String>,
    pub owner: String,
    pub instructions: String,
    pub source: String,
    pub reviewed: String,
    #[serde(default)]
    pub repository: String,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Knowledge {
    pub recipe: String,
    pub owner: String,
    pub instructions: String,
    pub source: String,
    pub evidence: String,
    pub repository: String,
    pub revision: String,
}
pub static DATABASE: LazyLock<Database> = LazyLock::new(|| {
    let db: Database =
        serde_json::from_str(include_str!("../data/recipes.json")).expect("bundled recipes");
    assert_eq!(db.schema, 1);
    db
});
static PATTERNS: LazyLock<Vec<regex::Regex>> = LazyLock::new(|| {
    DATABASE
        .recipes
        .iter()
        .map(|r| regex::Regex::new(&r.name_pattern).expect("recipe pattern"))
        .collect()
});

pub fn detect(a: &App) -> Option<Knowledge> {
    if a.protected {
        return None;
    }
    if a.kind == "registry"
        && a.key.splitn(4, ':').nth(3).is_some_and(|key| {
            key.strip_prefix("Steam App ").is_some_and(|id| {
                !id.is_empty() && id.len() <= 12 && id.bytes().all(|b| b.is_ascii_digit())
            })
        })
    {
        return Some(Knowledge { recipe: "steam-game".into(), owner: "Steam".into(), instructions: "Open Steam Library, select this game, then Properties > Updates. Steam owns its downloads and removal. AppTide cannot independently confirm its latest build.".into(), source: "https://help.steampowered.com/en/faqs/view/71AB-698D-57EB-178C".into(), evidence: "Numeric Steam application ID in the uninstall registration".into(), revision: DATABASE.revision.clone(), ..Default::default() });
    }
    if a.kind == "appx" && a.extra.get("signatureKind").and_then(|v| v.as_str()) == Some("Store") {
        return Some(Knowledge { recipe: "microsoft-store".into(), owner: "Microsoft Store".into(), instructions: "Open Microsoft Store > Library to check downloads and updates. Store-signed identity identifies the channel; it does not prove an update is available.".into(), source: "https://support.microsoft.com/en-us/windows/apps/turn-on-automatic-app-updates".into(), evidence: "Windows reports a Store-signed AppX package".into(), revision: DATABASE.revision.clone(), ..Default::default() });
    }
    if !["registry", "portable"].contains(&a.kind.as_str()) {
        return None;
    }
    let matches: Vec<_> = DATABASE
        .recipes
        .iter()
        .zip(PATTERNS.iter())
        .filter(|(r, p)| {
            p.is_match(&a.name)
                && r.publishers
                    .iter()
                    .any(|v| v.eq_ignore_ascii_case(a.publisher.trim()))
        })
        .collect();
    if matches.len() != 1 {
        return None;
    }
    let r = matches[0].0;
    Some(Knowledge {
        recipe: r.id.clone(),
        owner: r.owner.clone(),
        instructions: r.instructions.clone(),
        source: r.source.clone(),
        evidence: format!(
            "Anchored product name and exact publisher; recipe reviewed {}",
            r.reviewed
        ),
        repository: r.repository.clone(),
        revision: DATABASE.revision.clone(),
    })
}
pub fn enrich(c: &mut Catalog) {
    for a in c.apps.values_mut().filter(|a| a.present) {
        a.knowledge = detect(a);
        let stale = a.extra.get("watch").is_some_and(|w| {
            w["association"] == "bundled-recipe"
                && a.knowledge
                    .as_ref()
                    .is_none_or(|k| w["repository"] != k.repository)
        });
        if stale {
            a.extra.remove("watch");
        }
        if let Some(w) = a.extra.get_mut("watch")
            && let Some(latest) = w["latest"].as_str()
        {
            w["status"] = crate::releases::compare(&a.version, latest).into();
        }
        let receipts: Vec<_> = c
            .receipts
            .iter()
            .filter(|r| {
                a.winget_id
                    .as_deref()
                    .is_some_and(|id| r["packageId"] == id)
            })
            .collect();
        a.extra.insert("learning".into(),serde_json::json!({"schema":1,"versionSightings":a.versions.len(),"verifiedOperations":receipts.iter().filter(|r|r["status"]=="verified").count(),"unverifiedOperations":receipts.iter().filter(|r|r["status"]=="unverified" || r["status"]=="failed").count(),"recipeRevision":DATABASE.revision,"executionAuthority":"WinGet identity and explicit consent only"}));
    }
}
pub fn route(a: &App) -> &str {
    if a.protected {
        "Protected component"
    } else if a.ambiguous {
        "Review identity"
    } else if a.winget_id.is_some() {
        "WinGet"
    } else if a
        .extra
        .get("watch")
        .and_then(|v| v["repository"].as_str())
        .is_some_and(crate::releases::valid_repository)
    {
        "Release watch"
    } else if let Some(k) = &a.knowledge {
        &k.owner
    } else {
        "Needs research"
    }
}
pub fn coverage(c: &Catalog) -> serde_json::Value {
    let mut routes = std::collections::BTreeMap::<String, usize>::new();
    for a in c.apps.values().filter(|a| a.present) {
        *routes.entry(route(a).into()).or_default() += 1;
    }
    serde_json::json!({"databaseRevision":DATABASE.revision,"productRecipes":DATABASE.recipes.len(),"channelRules":2,"routes":routes,"beyondWinget":c.apps.values().filter(|a|a.present && !a.protected && a.winget_id.is_none() && a.knowledge.is_some()).count()})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recipes_are_unique_reviewed_and_do_not_grant_execution() {
        let mut ids = std::collections::BTreeSet::new();
        for (r, p) in DATABASE.recipes.iter().zip(PATTERNS.iter()) {
            assert!(ids.insert(&r.id));
            assert!(r.source.starts_with("https://"));
            assert!(!r.reviewed.is_empty());
            assert!(!r.publishers.is_empty());
            assert!(p.as_str().contains('^') && p.as_str().ends_with('$'));
        }
        let mut a = App {
            name: "meetily".into(),
            publisher: "meetily".into(),
            kind: "registry".into(),
            present: true,
            ..Default::default()
        };
        a.knowledge = detect(&a);
        assert!(a.knowledge.is_some());
        assert!(!crate::catalog::actionable(&a));
        a.publisher = "Unrelated vendor".into();
        assert!(detect(&a).is_none());
        a.publisher = "meetily".into();
        a.name = "meetily Private Fork".into();
        assert!(detect(&a).is_none());
    }
    #[test]
    fn channel_detection_does_not_trust_names_or_sideloads() {
        let mut a = App {
            kind: "appx".into(),
            name: "Microsoft.Test".into(),
            ..Default::default()
        };
        assert!(detect(&a).is_none());
        a.extra.insert("signatureKind".into(), "Developer".into());
        assert!(detect(&a).is_none());
        a.extra.insert("signatureKind".into(), "Store".into());
        assert_eq!(detect(&a).unwrap().owner, "Microsoft Store");
        a.kind = "registry".into();
        a.key = "registry:LocalMachine:Registry64:Steam App 123".into();
        assert_eq!(detect(&a).unwrap().owner, "Steam");
        a.key.push_str(" bad");
        assert!(detect(&a).is_none());
    }
    #[test]
    fn cached_releases_follow_installed_versions_and_recipe_identity() {
        let mut c = Catalog::default();
        c.observe(vec![App {
            uid: "test".into(),
            name: "meetily".into(),
            publisher: "meetily".into(),
            kind: "registry".into(),
            version: "0.4.0".into(),
            ..Default::default()
        }]);
        enrich(&mut c);
        crate::releases::apply(
            c.apps.get_mut("test").unwrap(),
            &serde_json::json!({"repository":"Zackriya-Solutions/meetily","latest":"v0.4.1"}),
        );
        assert!(crate::releases::newer(&c.apps["test"]));
        c.apps.get_mut("test").unwrap().version = "0.4.1".into();
        enrich(&mut c);
        assert!(!crate::releases::newer(&c.apps["test"]));
        c.apps.get_mut("test").unwrap().publisher = "Other".into();
        enrich(&mut c);
        assert!(crate::releases::repository(&c.apps["test"]).is_none());
    }
}
