use regex::Regex;
use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WikiLinkMatch {
    pub name: String,
    pub start: usize,
    pub end: usize,
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
}
