use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use recsys_core::{Item, Scraper};
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};


const BASE_URL: &str = "http://export.arxiv.org/api/query";
const MIN_REQUEST_GAP: Duration = Duration::from_secs(3);

pub struct ArxivScraper {
    client: reqwest::blocking::Client,
    throttle_file: std::path::PathBuf,
    last_request: Mutex<Option<SystemTime>>,
}

impl ArxivScraper {
    pub fn new() -> anyhow::Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("recsys-rs (https://github.com/alexcatdogiscool/recsys-rs)")
            .build()?;
        Ok(Self {
            client,
            throttle_file: std::env::temp_dir().join("recsys_arxiv_last_request"),
            last_request: Mutex::new(None),
        })
    }

    // blocks 2 scrape attempts from ahppening too quickly as to not annoy arxiv...
    // may be overkill
    fn throttle(&self) {
        let disk_last = fs::read_to_string(&self.throttle_file)
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|secs| UNIX_EPOCH + Duration::from_secs(secs));

        let mut guard = self.last_request.lock().unwrap();
        let last = match (*guard, disk_last) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        if let Some(last) = last {
            if let Ok(elapsed) = SystemTime::now().duration_since(last) {
                if elapsed < MIN_REQUEST_GAP {
                    std::thread::sleep(MIN_REQUEST_GAP - elapsed);
                }
            }
        }

        let now = SystemTime::now();
        *guard = Some(now);
        if let Ok(secs) = now.duration_since(UNIX_EPOCH) {
            let _ = fs::write(&self.throttle_file, secs.as_secs().to_string());
        }
    }

    fn fetch(&self, params: &[(&str, String)]) -> anyhow::Result<String> {
        self.throttle();
        let resp = self
            .client
            .get(BASE_URL)
            .query(params)
            .send()?
            .error_for_status()?;
        Ok(resp.text()?)
    }
}

impl Scraper for ArxivScraper {
    type Representation = Vec<String>;

    fn source_name(&self) -> &str {
        "arxiv"
    }

    fn query_style_hint(&self) -> &str {
        "formal academic phrasing; technical terminology from paper titles/abstracts"
    }

    fn scrape(&mut self, keywords: &Vec<String>, limit: u32) -> anyhow::Result<Vec<Item>> {

        let search_query = keywords
            .iter()
            .map(|k| format!("all:{k}"))
            .collect::<Vec<_>>()
            .join(" OR ");

        let max_results = limit.min(2000);

        let body = self.fetch(&[
            ("search_query", search_query),
            ("start", "0".to_string()),
            ("max_results", max_results.to_string()),
        ])?;

        parse_feed(&body)
    }

    fn get(&self, id: &str) -> anyhow::Result<Option<Item>> {
        let body = self.fetch(&[("id_list", id.to_string())])?;
        let mut items = parse_feed(&body)?;
        Ok(items.pop())
    }
}


fn parse_feed(xml: &str) -> anyhow::Result<Vec<Item>> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut items = Vec::new();
    let mut buf = Vec::new();

    let mut in_entry = false;
    let mut cur_tag: Option<String> = None;
    let mut id = String::new();
    let mut title = String::new();
    let mut summary = String::new();
    let mut authors: Vec<String> = Vec::new();
    let mut in_author = false;
    let mut pdf_url: Option<String> = None;
    let mut primary_category: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Eof => break,
            Event::Start(e) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                match name.as_str() {
                    "entry" => {
                        in_entry = true;
                        id.clear();
                        title.clear();
                        summary.clear();
                        authors.clear();
                        pdf_url = None;
                        primary_category = None;
                    }
                    "author" => in_author = true,
                    "link" if in_entry => {
                        let mut rel = String::new();
                        let mut link_title = String::new();
                        let mut href = String::new();
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.local_name().as_ref()).to_string();
                            let val = attr.unescape_value()?.to_string();
                            match key.as_str() {
                                "rel" => rel = val,
                                "title" => link_title = val,
                                "href" => href = val,
                                _ => {}
                            }
                        }
                        if rel == "related" && link_title == "pdf" {
                            pdf_url = Some(href);
                        }
                    }
                    "primary_category" if in_entry => {
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.local_name().as_ref()).to_string();
                            if key == "term" {
                                primary_category = Some(attr.unescape_value()?.to_string());
                            }
                        }
                    }
                    _ => {}
                }
                cur_tag = Some(name);
            }
            Event::Empty(e) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                if in_entry && name == "link" {
                    let mut rel = String::new();
                    let mut link_title = String::new();
                    let mut href = String::new();
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.local_name().as_ref()).to_string();
                        let val = attr.unescape_value()?.to_string();
                        match key.as_str() {
                            "rel" => rel = val,
                            "title" => link_title = val,
                            "href" => href = val,
                            _ => {}
                        }
                    }
                    if rel == "related" && link_title == "pdf" {
                        pdf_url = Some(href);
                    }
                } else if in_entry && name == "primary_category" {
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.local_name().as_ref()).to_string();
                        if key == "term" {
                            primary_category = Some(attr.unescape_value()?.to_string());
                        }
                    }
                }
            }
            Event::Text(e) => {
                let text = e.unescape()?.trim().to_string();
                if text.is_empty() {
                    continue;
                }
                match cur_tag.as_deref() {
                    Some("id") if in_entry && !in_author => id = text,
                    Some("title") if in_entry => title = text,
                    Some("summary") if in_entry => summary = text,
                    Some("name") if in_author => authors.push(text),
                    _ => {}
                }
            }
            Event::End(e) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                match name.as_str() {
                    "entry" => {
                        in_entry = false;
                        if id.contains("/api/errors") {
                            return Err(anyhow::anyhow!("arXiv API error: {summary}"));
                        }
                        let arxiv_id = id.rsplit('/').next().unwrap_or(&id).to_string();
                        items.push(Item {
                            id: arxiv_id,
                            source: "arxiv".to_string(),
                            title: title.clone(),
                            text: format!("{title}\n\n{summary}"),
                            url: pdf_url.clone().or_else(|| Some(id.clone())),
                            metadata: {
                                let mut m = HashMap::new();
                                if !authors.is_empty() {
                                    m.insert("authors".to_string(), serde_json::json!(authors));
                                }
                                if let Some(cat) = &primary_category {
                                    m.insert("primary_category".to_string(), serde_json::json!(cat));
                                }
                                m
                            },
                        });
                    }
                    "author" => in_author = false,
                    _ => {}
                }
                cur_tag = None;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(items)
}