
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
    CentroidScorer,
};
use recsys_embed_local::{
    OllamaApi,
};
use std::fs;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashMap;
use std::io::{self, Write};







fn main() -> anyhow::Result<()> {
    



    let mut engine = EngineBuilder::new()
    .scorer(
        CentroidScorer {
            ignore_penalty: 0.0,
            explore_fraction: 0.0
        }
    )
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
        struct FakeScraper2 {
            rand: StdRng,
        }
        impl Scraper for FakeScraper2 {
            type Representation = u32;
            fn source_name(&self) -> &str {
                "FakeScraper"
            }

            fn scrape(&mut self, keywords: &Self::Representation, limit: u32) -> anyhow::Result<Vec<Item>> {
                let contents: String = fs::read_to_string("test.txt")?;
                let words: Vec<&str> = contents.split(".").collect();

                let mut items: Vec<Item> = Vec::new();
                for i in 0..limit {
                    let w = words[self.rand.random_range(0..words.len()-1)];
                    //let w = words[i as usize];
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
        let scrap: FakeScraper2 = FakeScraper2 {
            rand: StdRng::seed_from_u64(0),
        };
        scrap
    })
    .build();

    /*
    let seed_item = Item {
        id: "".to_string(),
        source: "".to_string(),
        title: "".to_string(),
        text: "i am eating".to_string(),
        url: None,
        metadata: HashMap::new(),
    };

    let seed_item2 = Item {
        id: "".to_string(),
        source: "".to_string(),
        title: "".to_string(),
        text: "Evaluating the C++ library ecosystem".to_string(),
        url: None,
        metadata: HashMap::new(),
    };
    */

    let mut profile = Profile {
        engage_history: Vec::new(),
        ignore_history: Vec::new(),
    };

    loop {
        let recomendations = engine.recommend(&profile, 20).expect(":(");
        let (best_rec, best_score) = &recomendations[0];

        println!("{} | ({})", best_rec.text, best_score);
        print!("engage (e) or ignore (i): ");
        io::stdout().flush().unwrap();

        let mut input: String = String::new();

        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input");
        input = input.trim().to_string();

        if input == "e" {
            profile.engage_history.push(best_rec.clone());
        }
        else if input == "i" {
            profile.ignore_history.push(best_rec.clone());
        }


    }

    

    

    //println!("{} | {}", recomendations.len());

    Ok(())

}

