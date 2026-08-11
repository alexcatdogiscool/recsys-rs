use recsys_core::{
    Item,
    Scraper,
};
use std::collections::HashMap;
use serde::Deserialize;

const BASE_URL: &str = "https://hn.algolia.com/api/v1";

pub struct HnScraper {
    client: reqwest::blocking::Client,
    /// "search" (relevance-ranked) or "search_by_date" (newest-first).
    sort: HnSort,
}
 
pub enum HnSort {
    Relevance,
    Date,
}
 
impl HnScraper {
    pub fn new(sort: HnSort) -> anyhow::Result<Self> {
        let client = reqwest::blocking::Client::builder()
            // Not required by Algolia, but good practice per arXiv-style API etiquette
            // and gives HN something to identify you by if they ever reach out.
            .user_agent("recsys-rs (https://github.com/alexcatdogiscool/recsys-rs)")
            .build()?;
        Ok(Self { client, sort })
    }
}

#[derive(Deserialize)]
struct SearchResponse {
    hits: Vec<Hit>,
}
 
#[derive(Deserialize)]
struct Hit {
    #[serde(rename = "objectID")]
    object_id: String,
    title: Option<String>,
    url: Option<String>,
    author: Option<String>,
    points: Option<i64>,
    num_comments: Option<i64>,
    created_at: Option<String>,
    story_text: Option<String>,
    comment_text: Option<String>,
}

impl Hit {
    fn into_item(self) -> Item {
        let text = self
            .title
            .clone()
            .into_iter()
            .chain(self.story_text)
            .chain(self.comment_text)
            .collect::<Vec<_>>()
            .join("\n\n");
 
        let mut metadata = HashMap::new();
        if let Some(p) = self.points {
            metadata.insert("points".to_string(), serde_json::json!(p));
        }
        if let Some(c) = self.num_comments {
            metadata.insert("num_comments".to_string(), serde_json::json!(c));
        }
        if let Some(a) = &self.author {
            metadata.insert("author".to_string(), serde_json::json!(a));
        }
        if let Some(t) = &self.created_at {
            metadata.insert("created_at".to_string(), serde_json::json!(t));
        }
        if let Some(u) = &self.url {
            metadata.insert("url".to_string(), serde_json::json!(u));
        }
 
        Item {
            id: self.object_id.clone(),
            source: "hackernews".to_string(),
            title: self.title.unwrap_or_default(),
            text,
            url: self
                .url
                .or_else(|| Some(format!("https://news.ycombinator.com/item?id={}", self.object_id))),
            metadata,
        }
    }
}


impl Scraper for HnScraper {
    type Representation = Vec<String>;
 
    fn source_name(&self) -> &str {
        "hackernews"
    }
 
    fn query_style_hint(&self) -> &str {
        ""
    }
 
    fn scrape(&mut self, keywords: &Vec<String>, limit: u32) -> anyhow::Result<Vec<Item>> {
        let endpoint = match self.sort {
            HnSort::Relevance => "search",
            HnSort::Date => "search_by_date",
        };
        let query = keywords.join(" ");
        let hits_per_page = limit.min(1000);
 
        let resp = self
            .client
            .get(format!("{BASE_URL}/{endpoint}"))
            .query(&[
                ("query", query.as_str()),
                ("tags", "story"),
                ("hitsPerPage", &hits_per_page.to_string()),
            ])
            .send()?
            .error_for_status()?
            .json::<SearchResponse>()?;
 
        Ok(resp.hits.into_iter().map(Hit::into_item).collect())
    }
 
    fn get(&self, id: &str) -> anyhow::Result<Option<Item>> {
        let resp = self
            .client
            .get(format!("{BASE_URL}/items/{id}"))
            .send()?;
 
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let hit = resp.error_for_status()?.json::<Hit>()?;
        Ok(Some(hit.into_item()))
    }
}