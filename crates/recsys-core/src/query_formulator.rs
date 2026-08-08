use crate::Item;

pub trait QueryFormulator {
    /// Turn recent profile context into per-scraper search keywords.
    fn formulate(&self, topic_context: &str, style_hint: &str) -> anyhow::Result<Vec<String>>;
}

pub trait QueryBuilder<Q> {
    fn build_query(&self, engage_history: &[Item], style_hint: &str) -> anyhow::Result<Q>;
}