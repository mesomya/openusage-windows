use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestLine {
    #[serde(rename = "type")]
    pub line_type: String,
    pub label: String,
    pub scope: String,
    /// Lower number = higher priority for primary metric selection.
    /// Only progress lines with primary_order are candidates.
    pub primary_order: Option<u32>,
    /// Marks this line as the provider's recurring-period metric for the
    /// menubar metric preference. Currently only "weekly" is recognized.
    pub period: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginLink {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub entry: String,
    pub icon: String,
    pub brand_color: Option<String>,
    pub lines: Vec<ManifestLine>,
    #[serde(default)]
    pub links: Vec<PluginLink>,
}

#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub plugin_dir: PathBuf,
    pub entry_script: String,
    pub icon_data_url: String,
}

pub fn load_plugins_from_dir(plugins_dir: &std::path::Path) -> Vec<LoadedPlugin> {
    let mut plugins = Vec::new();
    let entries = match std::fs::read_dir(plugins_dir) {
        Ok(e) => e,
        Err(_) => return plugins,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("plugin.json");
        if !manifest_path.exists() {
            continue;
        }
        if let Ok(p) = load_single_plugin(&path) {
            plugins.push(p);
        }
    }

    plugins.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
    plugins
}

/// Label of the progress line marked `"period": "weekly"`, if any.
/// Drives the menubar weekly-metric preference; first match wins.
pub fn weekly_candidate(lines: &[ManifestLine]) -> Option<&str> {
    lines
        .iter()
        .find(|line| line.line_type == "progress" && line.period.as_deref() == Some("weekly"))
        .map(|line| line.label.as_str())
}

fn load_single_plugin(
    plugin_dir: &std::path::Path,
) -> Result<LoadedPlugin, Box<dyn std::error::Error>> {
    let manifest_path = plugin_dir.join("plugin.json");
    let manifest_text = std::fs::read_to_string(&manifest_path)?;
    let mut manifest: PluginManifest = serde_json::from_str(&manifest_text)?;
    manifest.links = sanitize_plugin_links(&manifest.id, std::mem::take(&mut manifest.links));

    // Validate primary_order / period: only progress lines can carry them,
    // and period currently only recognizes "weekly".
    for line in manifest.lines.iter() {
        if line.primary_order.is_some() && line.line_type != "progress" {
            log::warn!(
                "plugin {} line '{}' has primaryOrder but type is '{}'; will be ignored",
                manifest.id,
                line.label,
                line.line_type
            );
        }
        if let Some(period) = line.period.as_deref() {
            if line.line_type != "progress" {
                log::warn!(
                    "plugin {} line '{}' has period but type is '{}'; will be ignored",
                    manifest.id,
                    line.label,
                    line.line_type
                );
            } else if period != "weekly" {
                log::warn!(
                    "plugin {} line '{}' has unsupported period '{}'; only \"weekly\" is recognized",
                    manifest.id,
                    line.label,
                    period
                );
            }
        }
    }

    if manifest.entry.trim().is_empty() {
        return Err("plugin entry field cannot be empty".into());
    }
    let entry_rel = Path::new(&manifest.entry);
    if entry_rel.is_absolute() {
        return Err("plugin entry must be a relative path".into());
    }
    // Guard against path traversal with a component check rather than
    // canonicalize()+starts_with. On Windows, canonicalize() can resolve a
    // directory and a file beneath it to *different* roots when the path runs
    // through filesystem virtualization / redirected (roaming) folders — e.g.
    // a dir canonicalizes under AppData\Roaming while the file inside it
    // canonicalizes under AppData\Local\Packages\...\LocalCache\Roaming. That
    // made every legitimate entry look like it escaped its plugin dir, so no
    // plugins loaded. Rejecting `..`/absolute/root components on the relative
    // entry is portable and sufficient: the bundled tree contains no symlinks
    // (copy_dir_recursive skips them).
    let escapes = entry_rel.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    });
    if escapes {
        return Err("plugin entry must remain within plugin directory".into());
    }

    let entry_path = plugin_dir.join(entry_rel);
    if !entry_path.is_file() {
        return Err("plugin entry must be a file".into());
    }

    let entry_script = std::fs::read_to_string(&entry_path)?;

    let icon_file = plugin_dir.join(&manifest.icon);
    let icon_bytes = std::fs::read(&icon_file)?;
    let icon_data_url = format!("data:image/svg+xml;base64,{}", STANDARD.encode(&icon_bytes));

    Ok(LoadedPlugin {
        manifest,
        plugin_dir: plugin_dir.to_path_buf(),
        entry_script,
        icon_data_url,
    })
}

fn sanitize_plugin_links(plugin_id: &str, links: Vec<PluginLink>) -> Vec<PluginLink> {
    links
        .into_iter()
        .filter_map(|link| {
            let label = link.label.trim().to_string();
            let url = link.url.trim().to_string();

            if label.is_empty() || url.is_empty() {
                log::warn!(
                    "plugin {} has link with empty label/url; skipping",
                    plugin_id
                );
                return None;
            }
            if !(url.starts_with("https://") || url.starts_with("http://")) {
                log::warn!(
                    "plugin {} link '{}' has non-http(s) url '{}'; skipping",
                    plugin_id,
                    label,
                    url
                );
                return None;
            }

            Some(PluginLink { label, url })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_manifest(json: &str) -> PluginManifest {
        serde_json::from_str::<PluginManifest>(json).expect("manifest parse failed")
    }

    fn write_plugin_dir(entry: &str) -> PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "openusage-manifest-{}-{}",
            std::process::id(),
            suffix
        ));
        std::fs::create_dir_all(&dir).expect("create plugin dir");
        std::fs::write(
            dir.join("plugin.json"),
            format!(
                r##"{{"schemaVersion":1,"id":"t","name":"T","version":"0.0.1","entry":"{}","icon":"icon.svg","brandColor":null,"lines":[]}}"##,
                entry
            ),
        )
        .expect("write manifest");
        std::fs::write(dir.join("plugin.js"), "globalThis.__x = 1").expect("write entry");
        std::fs::write(
            dir.join("icon.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"></svg>"#,
        )
        .expect("write icon");
        dir
    }

    #[test]
    fn load_single_plugin_loads_normal_relative_entry() {
        let dir = write_plugin_dir("plugin.js");
        let loaded = load_single_plugin(&dir).expect("plugin should load");
        assert_eq!(loaded.manifest.id, "t");
        assert_eq!(loaded.entry_script, "globalThis.__x = 1");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_single_plugin_rejects_parent_dir_traversal_in_entry() {
        let dir = write_plugin_dir("../evil.js");
        let err = load_single_plugin(&dir).expect_err("traversal must be rejected");
        assert!(err.to_string().contains("remain within plugin directory"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_single_plugin_rejects_absolute_entry() {
        let abs = if cfg!(windows) { "C:/evil.js" } else { "/evil.js" };
        let dir = write_plugin_dir(abs);
        let err = load_single_plugin(&dir).expect_err("absolute entry must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("relative path") || msg.contains("remain within plugin directory"),
            "unexpected error: {msg}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn primary_order_is_none_by_default() {
        let manifest = parse_manifest(
            r#"
            {
              "schemaVersion": 1,
              "id": "x",
              "name": "X",
              "version": "0.0.1",
              "entry": "plugin.js",
              "icon": "icon.svg",
              "brandColor": null,
              "lines": [
                { "type": "progress", "label": "A", "scope": "overview" }
              ]
            }
            "#,
        );
        assert_eq!(manifest.lines.len(), 1);
        assert!(manifest.lines[0].primary_order.is_none());
        assert!(manifest.links.is_empty());
    }

    #[test]
    fn primary_order_parsed_correctly() {
        let manifest = parse_manifest(
            r#"
            {
              "schemaVersion": 1,
              "id": "x",
              "name": "X",
              "version": "0.0.1",
              "entry": "plugin.js",
              "icon": "icon.svg",
              "brandColor": null,
              "lines": [
                { "type": "progress", "label": "A", "scope": "overview", "primaryOrder": 1 },
                { "type": "progress", "label": "B", "scope": "overview", "primaryOrder": 2 },
                { "type": "progress", "label": "C", "scope": "overview" }
              ]
            }
            "#,
        );

        assert_eq!(manifest.lines[0].primary_order, Some(1));
        assert_eq!(manifest.lines[1].primary_order, Some(2));
        assert!(manifest.lines[2].primary_order.is_none());
    }

    #[test]
    fn primary_candidates_sorted_by_order() {
        let manifest = parse_manifest(
            r#"
            {
              "schemaVersion": 1,
              "id": "x",
              "name": "X",
              "version": "0.0.1",
              "entry": "plugin.js",
              "icon": "icon.svg",
              "brandColor": null,
              "lines": [
                { "type": "progress", "label": "Third", "scope": "overview", "primaryOrder": 3 },
                { "type": "progress", "label": "First", "scope": "overview", "primaryOrder": 1 },
                { "type": "progress", "label": "Second", "scope": "overview", "primaryOrder": 2 },
                { "type": "progress", "label": "None", "scope": "overview" }
              ]
            }
            "#,
        );

        // Extract candidates sorted by primary_order (same logic as lib.rs)
        let mut candidates: Vec<_> = manifest
            .lines
            .iter()
            .filter(|l| l.line_type == "progress" && l.primary_order.is_some())
            .collect();
        candidates.sort_by_key(|l| l.primary_order.unwrap());
        let labels: Vec<_> = candidates.iter().map(|l| l.label.as_str()).collect();

        assert_eq!(labels, vec!["First", "Second", "Third"]);
    }

    #[test]
    fn period_parsed_and_weekly_candidate_resolved() {
        let manifest = parse_manifest(
            r#"
            {
              "schemaVersion": 1,
              "id": "x",
              "name": "X",
              "version": "0.0.1",
              "entry": "plugin.js",
              "icon": "icon.svg",
              "brandColor": null,
              "lines": [
                { "type": "progress", "label": "Session", "scope": "overview", "primaryOrder": 1 },
                { "type": "progress", "label": "Weekly", "scope": "overview", "period": "weekly" }
              ]
            }
            "#,
        );

        assert!(manifest.lines[0].period.is_none());
        assert_eq!(manifest.lines[1].period.as_deref(), Some("weekly"));

        // Exercise the shipped resolver used by list_plugins.
        assert_eq!(weekly_candidate(&manifest.lines), Some("Weekly"));
    }

    #[test]
    fn weekly_candidate_absent_when_no_period() {
        let manifest = parse_manifest(
            r#"
            {
              "schemaVersion": 1,
              "id": "x",
              "name": "X",
              "version": "0.0.1",
              "entry": "plugin.js",
              "icon": "icon.svg",
              "brandColor": null,
              "lines": [
                { "type": "progress", "label": "Session", "scope": "overview", "primaryOrder": 1 }
              ]
            }
            "#,
        );

        assert_eq!(weekly_candidate(&manifest.lines), None);
    }

    #[test]
    fn weekly_candidate_first_match_wins() {
        // Precedence is intentionally first-match; lock it in so it can't drift silently.
        let manifest = parse_manifest(
            r#"
            {
              "schemaVersion": 1,
              "id": "x",
              "name": "X",
              "version": "0.0.1",
              "entry": "plugin.js",
              "icon": "icon.svg",
              "brandColor": null,
              "lines": [
                { "type": "progress", "label": "Weekly A", "scope": "overview", "period": "weekly" },
                { "type": "progress", "label": "Weekly B", "scope": "overview", "period": "weekly" }
              ]
            }
            "#,
        );

        assert_eq!(weekly_candidate(&manifest.lines), Some("Weekly A"));
    }

    #[test]
    fn weekly_candidate_ignores_unsupported_period() {
        // A typo'd period (e.g. "week") is not recognized; the provider keeps its primary metric.
        let manifest = parse_manifest(
            r#"
            {
              "schemaVersion": 1,
              "id": "x",
              "name": "X",
              "version": "0.0.1",
              "entry": "plugin.js",
              "icon": "icon.svg",
              "brandColor": null,
              "lines": [
                { "type": "progress", "label": "Weekly", "scope": "overview", "period": "week" }
              ]
            }
            "#,
        );

        assert_eq!(weekly_candidate(&manifest.lines), None);
    }

    #[test]
    fn links_are_parsed_when_present() {
        let manifest = parse_manifest(
            r#"
            {
              "schemaVersion": 1,
              "id": "x",
              "name": "X",
              "version": "0.0.1",
              "entry": "plugin.js",
              "icon": "icon.svg",
              "brandColor": null,
              "links": [
                { "label": "Status", "url": "https://status.example.com" },
                { "label": "Billing", "url": "https://example.com/billing" }
              ],
              "lines": [
                { "type": "progress", "label": "A", "scope": "overview", "primaryOrder": 1 }
              ]
            }
            "#,
        );

        assert_eq!(manifest.links.len(), 2);
        assert_eq!(manifest.links[0].label, "Status");
        assert_eq!(manifest.links[1].url, "https://example.com/billing");
    }

    #[test]
    fn sanitize_plugin_links_filters_invalid_entries() {
        let links = vec![
            PluginLink {
                label: " Status ".to_string(),
                url: " https://status.example.com ".to_string(),
            },
            PluginLink {
                label: " ".to_string(),
                url: "https://example.com".to_string(),
            },
            PluginLink {
                label: "Docs".to_string(),
                url: "ftp://example.com".to_string(),
            },
        ];

        let sanitized = sanitize_plugin_links("x", links);
        assert_eq!(sanitized.len(), 1);
        assert_eq!(sanitized[0].label, "Status");
        assert_eq!(sanitized[0].url, "https://status.example.com");
    }
}
