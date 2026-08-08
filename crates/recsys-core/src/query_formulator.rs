

pub trait QueryFormulator {
    /// Turn recent profile context into per-scraper search keywords.
    fn formulate(&self, topic_context: &str, style_hint: &str) -> anyhow::Result<Vec<String>>;
}