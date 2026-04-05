use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

pub fn tool_web_search(query: &str) -> String {
    let encoded = utf8_percent_encode(query, NON_ALPHANUMERIC).to_string();
    let url = format!("https://html.duckduckgo.com/html/?q={}", encoded);

    match ureq::get(&url)
        .set(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .call()
    {
        Ok(resp) => {
            let html: String = resp.into_string().unwrap_or_default();
            extract_search_results(&html)
        }
        Err(e) => format!("Search failed: {}", e),
    }
}

fn extract_search_results(html: &str) -> String {
    let mut results = Vec::new();
    let snippet_tag = "class=\"result__snippet\"";
    let mut pos = 0;

    while let Some(start) = html[pos..].find(snippet_tag) {
        let tag_start = pos + start;
        if let Some(gt) = html[tag_start..].find('>') {
            let content_start = tag_start + gt + 1;
            if let Some(end_tag) = html[content_start..].find("</a>") {
                let raw = &html[content_start..content_start + end_tag];
                let clean = strip_html_tags(raw).trim().to_string();
                if !clean.is_empty() && results.len() < 5 {
                    results.push(clean);
                }
                pos = content_start + end_tag + 4;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if results.is_empty() {
        return "No search results found.".to_string();
    }

    results
        .into_iter()
        .enumerate()
        .map(|(i, r)| format!("{}. {}", i + 1, r))
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_html_tags(s: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    result = result.replace("&amp;", "&");
    result = result.replace("&lt;", "<");
    result = result.replace("&gt;", ">");
    result = result.replace("&quot;", "\"");
    result = result.replace("&#39;", "'");
    result
}
