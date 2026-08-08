use std::collections::HashMap;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Item {
    pub id: String,           // stable unique id (url, or source-prefixed id)
    pub source: String,       // "reddit", "wikipedia", "scholar"
    pub title: String,
    pub text: String,         // canonical text used for embedding (title + body/abstract)
    pub url: Option<String>,
    pub metadata: HashMap<String, Value>, // source-specific extras: upvotes, citations, etc.
}