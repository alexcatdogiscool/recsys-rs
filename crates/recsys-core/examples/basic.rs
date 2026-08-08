
use recsys_core::{
    Item,
    Profile,
    Scraper,
    RecommendationEngine,
    EmbeddingProvider,
    QueryBuilder,
    ScoringStrategy,
    FeatureExtractor,
};
use std::fs;
use rand::Rng;
use std::collections::HashMap;

struct FakeScraper;
impl Scraper for FakeScraper {
    type Representation = u32;
    fn source_name(&self) -> &str {
        "FakeScraper"
    }

    fn scrape(&self, keywords: &Self::Representation, limit: u32) -> anyhow::Result<Vec<Item>> {
        let contents: String = fs::read_to_string("test.txt")?;
        let words: Vec<&str> = contents.split(" ").collect();

        let mut rand = rand::rng();

        let mut items: Vec<Item> = Vec::new();
        for i in 0..limit {
            let w = words[rand.random_range(0..words.len()-1)];
            items.push(
                Item {
                    id: "".to_string(),
                    source: "".to_string(),
                    title: "".to_string(),
                    text: w.to_string(),
                    url: None,
                    metadata: HashMap::new(),
                }
            );
        }
        Ok(items)
    }

    fn get(&self, id: &str) -> anyhow::Result<Option<Item>> {
        Ok(None)
    }
}

impl QueryBuilder<u32> for FakeScraper {
    fn build_query(&self, engage_history: &[Item], _style_hint: &str) -> anyhow::Result<u32> {
        // e.g. average length of what the user's engaged with so far
        if engage_history.is_empty() { return Ok(4); }
        let avg = engage_history.iter().map(|i| i.text.len() as u32).sum::<u32>() / engage_history.len() as u32;
        Ok(avg)
    }
}

struct FakeScorer;
impl ScoringStrategy for FakeScorer {
    type Representation = u32;

    fn score(
        &self,
        engage: &[Self::Representation],
        ignore: &[Self::Representation],
        candidates: &[(Item, Self::Representation)],
    ) -> Vec<(Item, f32)> {
        let mut engage_score: f32 = engage.iter()
            .map(|i| *i).sum::<u32>() as f32;

        
        engage_score = engage_score / ( if candidates.len() == 0 { 1 } else { candidates.len() } as f32);
        

        
        let scored: Vec<(Item, f32)> = candidates.iter().map(| tup | {
                let item = tup.0.clone();
                let rep = tup.1;
                return (item, (rep as f32 - engage_score).abs())
            }).collect();

        return scored;        
    }
}

impl FeatureExtractor<u32> for FakeScorer {
    fn extract(&self, item: &Item) -> anyhow::Result<u32> {
        Ok(item.text.len() as u32)
    }
}




fn main() -> anyhow::Result<()> {
    

    let engine = RecommendationEngine {
        extractor: FakeScorer,
        query_builder: FakeScraper,
        scorer: FakeScorer,
        scraper: FakeScraper,
    };

    let seed_item = Item {
        id: "".to_string(),
        source: "".to_string(),
        title: "".to_string(),
        text: "hello".to_string(),
        url: None,
        metadata: HashMap::new(),
    };

    let profile = Profile {
        engage_history: Vec::new(),//vec![seed_item],
        ignore_history: Vec::new(),
    };

    let recomendations = engine.recommend(&profile, 10).expect(":(");

    for r in &recomendations {
        println!("{} | {}", r.0.text, r.1);
    }

    //println!("{} | {}", recomendations.len());

    Ok(())

}

