pub use crate::Item;

pub trait Scraper {
    type Representation;
    fn source_name(&self) -> &str;

    /// Culture hint fed to the QueryFormulator, e.g. "informal, natural questions"
    fn query_style_hint(&self) -> &str { "" }

    fn scrape(&mut self, keywords: &Self::Representation, limit: u32) -> anyhow::Result<Vec<Item>>;
    fn get(&self, id: &str) -> anyhow::Result<Option<Item>>;
}