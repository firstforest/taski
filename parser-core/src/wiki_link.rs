use regex::Regex;
use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WikiLinkMatch {
    pub name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedName {
    pub name: String,
    pub is_journal: bool,
}

pub fn normalize_wiki_name(raw: &str) -> NormalizedName {
    let trimmed = raw.trim();
    let without_ext = trimmed
        .strip_suffix(".md")
        .unwrap_or(trimmed)
        .to_string();

    let date_re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    let is_journal = date_re.is_match(&without_ext);

    NormalizedName {
        name: without_ext,
        is_journal,
    }
}

pub fn parse_wiki_links(text: &str) -> Vec<WikiLinkMatch> {
    let re = Regex::new(r"\[\[([^\[\]|]+?)\]\]").unwrap();
    re.captures_iter(text)
        .filter_map(|caps| {
            let whole = caps.get(0)?;
            let name = caps.get(1)?.as_str().to_string();
            if name.is_empty() {
                return None;
            }
            Some(WikiLinkMatch {
                name,
                start: whole.start(),
                end: whole.end(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_link() {
        let text = "ここに [[foo]] があります";
        let got = parse_wiki_links(text);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "foo");
    }

    #[test]
    fn test_parse_link_with_md_extension() {
        let got = parse_wiki_links("[[bar.md]]");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "bar.md");
    }

    #[test]
    fn test_parse_multiple_links() {
        let got = parse_wiki_links("[[a]] と [[b]] と [[c]]");
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].name, "a");
        assert_eq!(got[1].name, "b");
        assert_eq!(got[2].name, "c");
    }

    #[test]
    fn test_parse_ignores_pipes_and_brackets() {
        assert_eq!(parse_wiki_links("[[foo|表示名]]").len(), 0);
        assert_eq!(parse_wiki_links("[[]]").len(), 0);
    }

    #[test]
    fn test_parse_returns_byte_offsets() {
        let text = "xx[[foo]]yy";
        let got = parse_wiki_links(text);
        assert_eq!(got[0].start, 2);
        assert_eq!(got[0].end, 9);
        assert_eq!(&text[got[0].start..got[0].end], "[[foo]]");
    }

    #[test]
    fn test_normalize_plain() {
        let got = normalize_wiki_name("foo");
        assert_eq!(got.name, "foo");
        assert!(!got.is_journal);
    }

    #[test]
    fn test_normalize_strips_md_extension() {
        let got = normalize_wiki_name("foo.md");
        assert_eq!(got.name, "foo");
        assert!(!got.is_journal);
    }

    #[test]
    fn test_normalize_detects_journal_date() {
        let got = normalize_wiki_name("2026-04-14");
        assert_eq!(got.name, "2026-04-14");
        assert!(got.is_journal);
    }

    #[test]
    fn test_normalize_trims_whitespace() {
        let got = normalize_wiki_name("  foo  ");
        assert_eq!(got.name, "foo");
    }
}
