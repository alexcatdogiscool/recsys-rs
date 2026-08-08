
use recsys_core::{Item, Profile, Scraper, RecommendationEngine, EmbeddingProvider, QueryFormulator, ScoringStrategy};
use std::fs;
use rand::Rng;
use std::collections::HashMap;

struct fakeScraper;

impl Scraper for fakeScraper {
    fn source_name(&self) -> &str {
        "fake"
    }

    fn query_style_hint(&self) -> &str { "fake-hint" }

    fn scrape(&self, keywords: &[String], limit: u32) -> anyhow::Result<Vec<Item>> {
        let fp = "test.txt";
        let res: anyhow::Result<Vec<Item>> = match fs::read_to_string(fp) {
            Ok(contents) => {
                let words: Vec<String> = contents.split(" ").map(|s| { s.to_string() }).collect();
                let mut rng = rand::rng();
                let mut chosen: Vec<Item> = Vec::new();
                for i in 0..limit {
                    let item = words[rng.random_range(0..words.len()-1)].clone();

                    chosen.push(
                        Item {
                            id: "none".to_string(),
                            source: "none".to_string(),
                            title: "none".to_string(),
                            text: item,
                            url: None,
                            metadata: HashMap::new()
                        }
                    );
                }

                Ok(chosen)

                

            }
            _ => {
                Ok(Vec::new())
            }
        };
        return res;
    }

    fn get(&self, id: &str) -> anyhow::Result<Option<Item>> {
        Ok(None)
    }
}

struct LengthScorer {
    pub ingnore_penalty: f32,
    pub exploration_fraction: f32,
}

impl ScoringStrategy for LengthScorer {
    fn score(&self, engage: &[Vec<f32>], ignore: &[Vec<f32>], candidates: &[(Item, Vec<f32>)]) -> Vec<(Item, f32)> {
        let scored: Vec<(Item, f32)> = candidates.iter().map(|item| {
            let s = (4.0 - item.0.text.len() as f32).abs();
            (item.0.clone(), s)
        }).collect();

        scored
    }

    fn score_simple(
        &self,
        profile_engage_vec: &Vec<Item>,
        profile_ignore_vec: &Vec<Item>,
        candidates: &[Item]
    ) -> Vec<(Item, f32)>
    {
        let mut prev_engage: f32 = profile_engage_vec.iter().map(|item| {
            item.text.len() as f32
        }).into_iter().sum();

        prev_engage = prev_engage / profile_engage_vec.len() as f32;

        let scored: Vec<(Item, f32)> = candidates.iter().map(|item| {
            let s = (prev_engage - item.text.len() as f32).abs();
            (item.clone(), s)
        }).collect();

        scored
    }
}

struct FakeEmbedder;
impl EmbeddingProvider for FakeEmbedder {
    fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        // deterministic fake: hash-ish, not random, so runs are reproducible
        Ok(vec![text.len() as f32, 1.0, 0.5])
    }
}

struct FakeFormulator;
impl QueryFormulator for FakeFormulator {
    fn formulate(&self, topic: &str, _hint: &str) -> anyhow::Result<Vec<String>> {
        Ok(vec![topic.to_string()])
    }
}

fn main() -> anyhow::Result<()> {
    let engine = RecommendationEngine {
        embedder: FakeEmbedder,
        formulator: FakeFormulator,
        scorer: LengthScorer { ingnore_penalty: 0.0, exploration_fraction: 0.2 },
        scrapers: vec![Box::new(fakeScraper)]
    };

    let profile = Profile {
        engage_history: Vec::new(),
        ignore_history: Vec::new(),
    };

    let recomendations = engine.recommend(&profile, 10).expect(":(");

    for r in &recomendations {
        println!("{} | {}", r.0.text, r.1);
    }

    //println!("{} | {}", recomendations.len());

    Ok(())

}

