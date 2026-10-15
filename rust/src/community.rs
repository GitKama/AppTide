//! Public recognition metadata, never installer instructions or installed versions.
use crate::catalog::App;
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::{BTreeSet, HashMap},
    sync::LazyLock,
};

#[derive(Deserialize)]
pub struct Database {
    pub generated: String,
    pub entries: Vec<Entry>,
}
#[derive(Deserialize)]
pub struct Entry {
    pub id: String,
    pub names: Vec<String>,
    pub publishers: Vec<String>,
    pub source: String,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub repository: String,
    #[serde(default)]
    pub description: String,
}
pub static DATABASE: LazyLock<Database> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../data/community.json")).expect("bundled public catalogue")
});
static IDS: LazyLock<HashMap<String, usize>> = LazyLock::new(|| {
    DATABASE
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.source == "WinGet")
        .map(|(i, e)| (e.id.to_lowercase(), i))
        .collect()
});
static NAMES: LazyLock<HashMap<String, Vec<usize>>> = LazyLock::new(|| {
    let mut index: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, e) in DATABASE.entries.iter().enumerate() {
        for name in &e.names {
            let ids = index.entry(crate::winget::normalize(name)).or_default();
            if !ids.contains(&i) {
                ids.push(i)
            }
        }
    }
    index
});
fn publisher(s: &str) -> String {
    let mut words: Vec<_> = s
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .collect();
    while words.last().is_some_and(|s| {
        [
            "inc",
            "incorporated",
            "llc",
            "ltd",
            "limited",
            "corp",
            "corporation",
            "gmbh",
        ]
        .contains(&s.as_str())
    }) {
        words.pop();
    }
    words.join("")
}
pub fn by_id(id: &str) -> Option<&'static Entry> {
    IDS.get(&id.to_lowercase()).map(|&i| &DATABASE.entries[i])
}
pub fn confirms(a: &App, id: &str) -> bool {
    by_id(id).is_some_and(|e| {
        let p = publisher(&a.publisher);
        !p.is_empty()
            && e.publishers.iter().any(|s| publisher(s) == p)
            && e.names.iter().any(|s| {
                crate::winget::normalize(s).trim_end_matches(" setup")
                    == crate::winget::normalize(&a.name)
            })
    })
}
pub fn enrich(a: &mut App) {
    a.extra.remove("community");
    if crate::steam::is_game(a)
        || a.protected
        || a.ambiguous
        || !["registry", "portable", "winget"].contains(&a.kind.as_str())
    {
        return;
    }
    let candidates: Vec<&Entry> = if let Some(e) = a.winget_id.as_deref().and_then(by_id) {
        vec![e]
    } else {
        let p = publisher(&a.publisher);
        NAMES
            .get(&crate::winget::normalize(&a.name))
            .into_iter()
            .flatten()
            .map(|&i| &DATABASE.entries[i])
            .filter(|e| !p.is_empty() && e.publishers.iter().any(|s| publisher(s) == p))
            .collect()
    };
    if candidates.is_empty() {
        return;
    }
    let mut repos: BTreeSet<_> = candidates
        .iter()
        .filter(|e| crate::releases::valid_repository(&e.repository))
        .map(|e| e.repository.as_str())
        .collect();
    // A matched record may itself name an exact GitHub project as its homepage.
    if repos.is_empty() && candidates.len() == 1 {
        let e = candidates[0];
        if let Some(repo) = github_homepage(&e.homepage) {
            repos.insert(repo);
        }
    }
    if candidates.len() != 1 && repos.len() != 1 {
        return;
    }
    let e = candidates[0];
    let repo = if repos.len() == 1 {
        *repos.first().unwrap()
    } else {
        ""
    };
    a.extra.insert("community".into(),json!({"id":e.id,"source":e.source,"repository":repo,"homepage":e.homepage,"description":e.description,"revision":DATABASE.generated,"evidence":if a.winget_id.is_some(){"Exact installed WinGet package ID"}else{"Exact product name and publisher"}}));
}
fn github_homepage(url: &str) -> Option<&str> {
    let s = url
        .strip_prefix("https://github.com/")?
        .trim_end_matches('/');
    crate::releases::valid_repository(s).then_some(s)
}
pub fn repositories() -> usize {
    static COUNT: LazyLock<usize> = LazyLock::new(|| {
        DATABASE
            .entries
            .iter()
            .filter(|e| !e.repository.is_empty())
            .map(|e| e.repository.to_lowercase())
            .collect::<BTreeSet<_>>()
            .len()
    });
    *COUNT
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_catalogue_is_broad_and_metadata_only() {
        assert!(DATABASE.entries.len() > 19_000);
        assert!(repositories() > 1_200);
        assert!(by_id("Notepad++.Notepad++").is_some());
        for e in &DATABASE.entries {
            assert!(e.repository.is_empty() || crate::releases::valid_repository(&e.repository));
        }
    }
    #[test]
    fn identities_need_publisher_and_manual_watch_wins() {
        let mut a = App {
            name: "Notepad++".into(),
            kind: "registry".into(),
            publisher: "Impostor".into(),
            ..Default::default()
        };
        enrich(&mut a);
        assert!(!a.extra.contains_key("community"));
        a.winget_id = Some("Notepad++.Notepad++".into());
        enrich(&mut a);
        assert_eq!(
            a.extra["community"]["repository"],
            "notepad-plus-plus/notepad-plus-plus"
        );
        crate::releases::set_watch(&mut a, "owner/custom").unwrap();
        enrich(&mut a);
        assert_eq!(crate::releases::repository(&a), Some("owner/custom"));
    }
}
