use crate::config::{App, Config};

#[derive(Clone)]
pub struct AppEntry {
    pub app: App,
    pub group: String,
}

pub fn flatten(config: &Config) -> Vec<AppEntry> {
    config
        .groups
        .iter()
        .flat_map(|group| {
            group.apps.iter().cloned().map(|app| AppEntry {
                app,
                group: group.name.clone(),
            })
        })
        .collect()
}

pub fn filter(entries: &[AppEntry], group: Option<&str>, query: &str) -> Vec<AppEntry> {
    let query = query.trim().to_lowercase();
    entries
        .iter()
        .filter(|entry| {
            let group_match = group.is_none_or(|group| entry.group == group);
            let query_match = query.is_empty()
                || entry.app.name.to_lowercase().contains(&query)
                || entry.group.to_lowercase().contains(&query);
            group_match && query_match
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::parse;

    #[test]
    fn filters_by_name_or_group_and_respects_tab() {
        let config = parse("groups:\n  - name: Dev\n    apps:\n      - name: Editor\n        path: C:\\Apps\\editor.exe\n      - name: Terminal\n        path: C:\\Apps\\terminal.exe\n  - name: Tools\n    apps:\n      - name: Editor\n        path: C:\\Apps\\other.exe\n").unwrap();
        let entries = flatten(&config);
        assert_eq!(filter(&entries, None, "dev").len(), 2);
        assert_eq!(filter(&entries, Some("Tools"), "editor").len(), 1);
        assert_eq!(filter(&entries, Some("Dev"), "term").len(), 1);
    }

    #[test]
    fn custom_group_named_all_is_distinct_from_the_all_tab() {
        let config = parse(
            "groups:\n  - name: All\n    apps:\n      - name: Custom\n        path: C:\\Apps\\custom.exe\n  - name: Tools\n    apps:\n      - name: Tool\n        path: C:\\Apps\\tool.exe\n",
        )
        .unwrap();
        let entries = flatten(&config);
        assert_eq!(filter(&entries, None, "").len(), 2);
        assert_eq!(filter(&entries, Some("All"), "").len(), 1);
    }
}
