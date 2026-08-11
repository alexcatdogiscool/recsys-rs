use arxiv_tools::{ArXiv, Paper, QueryParams};
use recsys_core::{
    Item,
    Scraper,
};
use std::collections::HashMap;


pub struct ArxivScraper;

impl Scraper for ArxivScraper {
    type Representation = Vec<String>;
    fn source_name(&self) -> &str {
        "ArXiv"
    }

    fn query_style_hint(&self) -> &str {
        ""
    }

    fn scrape(&mut self, keywords: &Self::Representation, limit: u32) -> anyhow::Result<Vec<Item>> {
        
        let args = QueryParams::and(
            vec![
                QueryParams::or({
                    let mut terms: Vec<_> = keywords.iter().map(|s| QueryParams::title(s)).collect();
                    if terms.is_empty() {
                        terms.push(QueryParams::title(" "));
                    }
                    terms
                })
            ]
        );
        let mut arxiv = ArXiv::from_args(args);
        arxiv.max_results(limit as u64);

        let rt = tokio::runtime::Runtime::new()?;

        let mut response = rt.block_on(arxiv.query());
        response.pop().ok_or_else(|| anyhow::anyhow!("Could not fetch from arvix"))?;

        let mut items: Vec<Item> = Vec::new();

        for res in response {
            items.push(
                Item {
                    id: res.id,
                    source: self.source_name().to_string(),
                    title: res.title,
                    text: res.abstract_text,
                    url: None,
                    metadata: HashMap::new(),
                }
            );
        }
        println!("number of scraped results: {}", items.len());
        Ok(items)
        //Ok(response)
    }

    fn get(&self, id: &str) -> anyhow::Result<Option<Item>> {
        Ok(None)
    }

}



