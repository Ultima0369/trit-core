//! arXiv preprint metadata — Atom XML feed.
//!
//! API: https://export.arxiv.org/api/query (public, no key, rate-limit ~1 req/3s).
//! Returns recent preprint metadata: title, authors, abstract, categories.
//!
//! ponytail: XML parsing via quick-xml; we extract only the fields we need
//! and skip the full Atom namespace resolution. Fail-safe on parse errors.

use async_trait::async_trait;
use chrono::Utc;
use std::time::Duration;

use crate::cache::http_client;
use crate::error::DataforgeError;
use crate::source::DataSource;
use crate::types::{DataCategory, RawSignal};

const CACHE_TTL: Duration = Duration::from_secs(3600);
const ARXIV_URL: &str = "https://export.arxiv.org/api/query";

/// ArXiv query parameters — we fetch recent papers in climate + ecology.
const ARXIV_QUERY: &str =
    "cat:physics.ao-ph+OR+cat:q-bio.PE+OR+all:climate+change+OR+all:biodiversity+loss";
const ARXIV_MAX_RESULTS: &str = "20";
const ARXIV_SORT_BY: &str = "submittedDate";
const ARXIV_SORT_ORDER: &str = "descending";

pub struct ArxivSource {
    pub http: reqwest::Client,
}

impl ArxivSource {
    pub fn new() -> Self {
        Self {
            http: http_client(),
        }
    }
}

impl Default for ArxivSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataSource for ArxivSource {
    fn name(&self) -> &str {
        "arXiv"
    }

    fn category(&self) -> DataCategory {
        DataCategory::ScientificResearch
    }

    async fn fetch(&self) -> Result<Vec<RawSignal>, DataforgeError> {
        let resp = self
            .http
            .get(ARXIV_URL)
            .query(&[
                ("search_query", ARXIV_QUERY),
                ("max_results", ARXIV_MAX_RESULTS),
                ("sortBy", ARXIV_SORT_BY),
                ("sortOrder", ARXIV_SORT_ORDER),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(DataforgeError::Unavailable(format!(
                "arXiv returned {}",
                resp.status()
            )));
        }

        let text = resp.text().await?;
        let entries = parse_arxiv_entries(&text);
        let now = Utc::now();

        let signals: Vec<RawSignal> = entries
            .into_iter()
            .map(|entry| {
                let id = RawSignal::compute_id(&entry.url, &now);
                let raw_content = format!(
                    "title:{} authors:{} categories:{} published:{} abstract:{}",
                    entry.title,
                    entry.authors.join("; "),
                    entry.categories.join(", "),
                    entry.published,
                    truncate(&entry.summary, 500)
                );
                RawSignal {
                    id,
                    source_url: entry.url,
                    source_name: "arXiv".into(),
                    category: DataCategory::ScientificResearch,
                    raw_content,
                    captured_at: now,
                    data_period: Some(entry.published),
                    location: None,
                }
            })
            .collect();

        Ok(signals)
    }

    fn fetch_interval(&self) -> Duration {
        CACHE_TTL
    }
}

/// A parsed arXiv entry (simplified Atom extraction).
struct ArxivEntry {
    title: String,
    authors: Vec<String>,
    summary: String,
    published: String,
    url: String,
    categories: Vec<String>,
}

/// Parse arXiv Atom XML into structured entries.
///
/// ponytail: regex-based extraction rather than full XML deserialization.
/// arXiv Atom feed has a stable enough structure that regex is more robust
/// against namespace variations than a schema-dependent parser. The cost
/// is zero — this runs once per hour with 20 entries.
fn parse_arxiv_entries(xml: &str) -> Vec<ArxivEntry> {
    // Split on <entry> tags to get individual entries
    let mut entries = Vec::new();
    let mut start = 0usize;

    while let Some(entry_start) = xml[start..].find("<entry>") {
        let abs_start = start + entry_start;
        let entry_end = xml[abs_start..]
            .find("</entry>")
            .map(|i| abs_start + i + "</entry>".len())
            .unwrap_or(xml.len());
        let entry_xml = &xml[abs_start..entry_end];

        let title = extract_tag(entry_xml, "title").unwrap_or_else(|| "unknown title".into());
        let authors = extract_all_tags(entry_xml, "name");
        let summary = extract_tag(entry_xml, "summary").unwrap_or_else(|| "".into());
        let published =
            extract_tag(entry_xml, "published").unwrap_or_else(|| "unknown date".into());
        let url = extract_tag(entry_xml, "id").unwrap_or_else(|| "unknown url".into());
        let categories = extract_all_tags(entry_xml, "category");
        // arXiv categories have their value in the `term` attribute, not text content.
        // If text extraction produced nothing, fall back to attribute extraction.
        let categories = if categories.is_empty() {
            extract_attrs(entry_xml, "category", "term")
        } else {
            categories
        };

        entries.push(ArxivEntry {
            title,
            authors,
            summary,
            published,
            url,
            categories,
        });

        start = entry_end;
    }

    entries
}

/// Extract text content of the first matching XML tag.
fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");

    // Handle attributes: <tag attr="val">
    let open_idx = xml.find(&open).or_else(|| {
        // Try <tag with attributes
        let pattern = format!("<{tag} ");
        xml.find(&pattern)
    })?;

    let content_start = if xml[open_idx..].starts_with(&open) {
        open_idx + open.len()
    } else {
        // Skip past the opening tag with attributes
        xml[open_idx..].find('>')? + open_idx + 1
    };

    let close_idx = xml[content_start..].find(&close)?;
    let content = &xml[content_start..content_start + close_idx];
    Some(unescape_xml(content))
}

/// Extract all text contents of a repeated XML tag.
fn extract_all_tags(xml: &str, tag: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut search_from = 0usize;
    let open_pat = format!("<{tag} ");
    let open_simple = format!("<{tag}>");

    while search_from < xml.len() {
        let (_open_idx, content_start) = if let Some(i) = xml[search_from..].find(&open_simple) {
            (search_from + i, search_from + i + open_simple.len())
        } else if let Some(i) = xml[search_from..].find(&open_pat) {
            let abs = search_from + i;
            let content = match xml[abs..].find('>') {
                Some(c) => abs + c + 1,
                None => break,
            };
            (abs, content)
        } else {
            break;
        };

        let close_tag = format!("</{tag}>");
        if let Some(close_idx) = xml[content_start..].find(&close_tag) {
            let content = &xml[content_start..content_start + close_idx];
            results.push(unescape_xml(content));
        }

        search_from = content_start;
    }

    results
}

/// Extract attribute values for a specific tag and attribute name.
fn extract_attrs(xml: &str, tag: &str, attr: &str) -> Vec<String> {
    let mut results = Vec::new();
    let pattern = format!("<{tag} ");
    let attr_pattern = format!("{attr}=\"");

    let mut search_from = 0usize;
    while let Some(tag_idx) = xml[search_from..].find(&pattern) {
        let abs_tag = search_from + tag_idx;
        let after_tag = &xml[abs_tag + pattern.len()..];
        if let Some(attr_idx) = after_tag.find(&attr_pattern) {
            let val_start = abs_tag + pattern.len() + attr_idx + attr_pattern.len();
            if let Some(val_end) = xml[val_start..].find('"') {
                results.push(xml[val_start..val_start + val_end].to_string());
            }
        }
        search_from = abs_tag + 1;
    }
    results
}

/// Basic XML unescaping of common entities.
fn unescape_xml(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace('\n', " ")
        .replace('\r', "")
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_entry() {
        let xml = r#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <id>http://arxiv.org/abs/2501.00001v1</id>
    <title>Climate Model Projections Under SSP5-8.5</title>
    <author><name>Smith, J.</name></author>
    <author><name>Jones, K.</name></author>
    <summary>A study of extreme climate scenarios.</summary>
    <published>2025-01-15T00:00:00Z</published>
    <category term="physics.ao-ph"/>
    <category term="q-bio.PE"/>
  </entry>
</feed>"#;

        let entries = parse_arxiv_entries(xml);
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert!(e.title.contains("Climate"));
        assert_eq!(e.authors.len(), 2);
        assert_eq!(e.categories.len(), 2);
    }

    #[test]
    fn parse_empty_feed() {
        let entries = parse_arxiv_entries("<feed></feed>");
        assert!(entries.is_empty());
    }

    #[test]
    fn truncate_long_text() {
        let s = "a".repeat(1000);
        assert_eq!(truncate(&s, 100).len(), 103); // 100 + "..."
    }

    #[test]
    fn source_metadata() {
        let source = ArxivSource::new();
        assert_eq!(source.name(), "arXiv");
        assert_eq!(source.category(), DataCategory::ScientificResearch);
    }
}
