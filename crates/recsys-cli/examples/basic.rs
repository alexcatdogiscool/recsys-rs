
use recsys_core::{
    Item,
    Profile,
    Scraper,
    RecommendationEngine,
    EmbeddingProvider,
    QueryBuilder,
    ScoringStrategy,
    FeatureExtractor,
    EngineBuilder,
};
use recsys_embed_local::{
    OllamaApi,
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

fn centroid(vecs: &[Vec<f32>]) -> Option<Vec<f32>> {
    if vecs.is_empty() {
        return None;
    }
    let dim = vecs[0].len();
    let mut sum = vec![0.0f32; dim];
    for v in vecs {
        for (i, x) in v.iter().enumerate() {
            sum[i] += x;
        }
    }
    let n = vecs.len() as f32;
    Some(sum.into_iter().map(|x| x / n).collect())
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) }
}


fn main() -> anyhow::Result<()> {
    
    /*
    let engine = RecommendationEngine {
        extractor: FakeScorer,
        query_builder: FakeScraper,
        scorer: FakeScorer,
        scraper: FakeScraper,
    };
    */



    let _engine = EngineBuilder::new()
    .scorer(FakeScorer)
    .extractor(FakeScorer)
    .query_builder(FakeScraper)
    .scraper(FakeScraper)
    .build();

    let engine = EngineBuilder::new()
    .scorer({
        struct CosineScorer {
            ignore_penalty: f32,
        };
        impl ScoringStrategy for CosineScorer {
            type Representation = Vec<f32>;
            fn score(
                &self,
                engage: &[Vec<f32>],
                ignore: &[Vec<f32>],
                candidates: &[(Item, Vec<f32>)]
            ) -> Vec<(Item, f32)> {
                let engage_centroid = centroid(engage);
                let ignore_centroid = centroid(ignore);

                let mut scored: Vec<(Item, f32)> = candidates.iter().map(|(item, vec)| {
                    let engage_sim = match &engage_centroid {
                        Some(c) => cosine(vec, c),
                        None => 0.0, // no engagement history yet — neutral, not NaN
                    };
                    let ignore_sim = match &ignore_centroid {
                        Some(c) => cosine(vec, c),
                        None => 0.0,
                    };
                    (item.clone(), engage_sim - self.ignore_penalty * ignore_sim)
                }).collect();

                scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                scored
            }
        }
        let cs: CosineScorer = CosineScorer {
            ignore_penalty: 0.2
        };
        cs
    })
    .extractor({
        struct OllamaEmbedder {
            api: OllamaApi,
        }
        impl FeatureExtractor<Vec<f32>> for OllamaEmbedder {
            fn extract(&self, item: &Item) -> anyhow::Result<Vec<f32>> {
                let rt = tokio::runtime::Runtime::new()?;
                let mut result = rt.block_on(self.api.clone().fetch(vec![item.text.clone()]))?;
                result.pop().ok_or_else(|| anyhow::anyhow!("empty embedding response"))
            }
        }
        let extract: OllamaEmbedder = OllamaEmbedder {
            api: OllamaApi {
                url: "http://localhost:11434/api/embed".to_string(),
                dim: 1024
            }
        };
        extract
    })
    .query_builder({
        struct FakeQB;
        impl QueryBuilder<u32> for FakeQB {
            fn build_query(&self, engage_history: &[Item], _style_hint: &str) -> anyhow::Result<u32> {
                // e.g. average length of what the user's engaged with so far
                if engage_history.is_empty() { return Ok(4); }
                let avg = engage_history.iter().map(|i| i.text.len() as u32).sum::<u32>() / engage_history.len() as u32;
                Ok(avg)
            }
        }
        FakeQB
    })
    .scraper({
        struct FakeScraper2;
        impl Scraper for FakeScraper2 {
            type Representation = u32;
            fn source_name(&self) -> &str {
                "FakeScraper"
            }

            fn scrape(&self, keywords: &Self::Representation, limit: u32) -> anyhow::Result<Vec<Item>> {
                let contents: String = fs::read_to_string("test.txt")?;
                let words: Vec<&str> = contents.split('\n').collect();

                let mut rand = rand::rng();

                let mut items: Vec<Item> = Vec::new();
                for i in 0..limit {
                    //let w = words[rand.random_range(0..words.len()-1)];
                    let w = words[i as usize];
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
        FakeScraper2
    })
    .build();

    let seed_item = Item {
        id: "".to_string(),
        source: "".to_string(),
        title: "".to_string(),
        text: "i am eating".to_string(),
        url: None,
        metadata: HashMap::new(),
    };

    let profile = Profile {
        engage_history: vec![seed_item],
        ignore_history: Vec::new(),
    };

    let recomendations = engine.recommend(&profile, 12).expect(":(");

    for r in &recomendations {
        println!("{} | {}", r.0.text, r.1);
    }

    //println!("{} | {}", recomendations.len());

    Ok(())

}

