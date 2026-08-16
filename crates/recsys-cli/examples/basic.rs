
use rand::seq::SliceRandom;
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
use recsys_scraper::{
    ArxivScraper,
    HnScraper,
    HnSort,
};
use recsys_embed_local::{
    OllamaApi,
};
use std::fs;
use rand::{Rng, SeedableRng};
use rand::rngs::{StdRng, ThreadRng};
use std::collections::HashMap;
use std::io::{self, Write};

use keyword_extraction::yake::{Yake, YakeParams};
use stop_words::{self, LANGUAGE};



struct YakeExtractor {
    stop_words: Vec<String>,
    punctuation: Vec<String>,
    seed_keywords: Vec<String>,
    num_keywords_extracted: usize,
    randomize_return_order: bool,
    rng: ThreadRng,
}
impl YakeExtractor {
    fn new(max_keywords: usize, randomize_return_order: bool) -> Self {
        YakeExtractor {
            stop_words: stop_words::get(LANGUAGE::English),
            punctuation: [
                ".", ",", ":", ";", "!", "?", "(", ")", "[", "]", "{", "}", "\"", "'",
            ].iter().map(|s| s.to_string()).collect(),
            seed_keywords: vec![
                    "ai".to_string(),
                    "mycelium".to_string(),
                    "type setting".to_string(),
                    "Ad hoc network".to_string(),
                    "VANET".to_string(),
                ],
            num_keywords_extracted: max_keywords,
            randomize_return_order,
            rng: rand::rng(),
        }
    }
}
impl QueryBuilder<Vec<String>> for YakeExtractor {
    fn build_query(&mut self, engage_history: &[Item], style_hint: &str) -> anyhow::Result<Vec<String>> {
        let mut res: Vec<String> = Vec::new();
        for item in engage_history {
            let yake = Yake::new(YakeParams::WithDefaults(item.text.as_str(), &self.stop_words));
            let ranked_keywords: Vec<String> = yake.get_ranked_keywords(self.num_keywords_extracted);
            res.extend(ranked_keywords);
        }

        if res.is_empty() {
            let rand = rand::random_range(0..self.seed_keywords.len()-1);
            res.push(self.seed_keywords[rand].clone());
        }

        if self.randomize_return_order {
            res.shuffle(&mut self.rng);
            res = res
                .iter()
                .take(self.num_keywords_extracted)
                .map(|k| format!("{k}"))
                .collect::<Vec<String>>();
        }
        

        println!("START extracted keywords");
        println!("{:?}", res);
        println!("END extracted keywords");

        Ok(res)
        
    }
}




fn main() -> anyhow::Result<()> {
    



    let mut engine = EngineBuilder::new()
    .scorer(
        CentroidScorer {
            ignore_penalty: 0.2,
            explore_fraction: 0.2
        }
    )
    .extractor(
        OllamaApi {
            url: "http://localhost:11434/api/embed".to_string(),
            model_name: "qwen3-embedding:0.6b".to_string(),
            dim: 1024,
            context_window_size: 32768,
        }
    )
    .query_builder({
        YakeExtractor::new(5, true)
    })
    .scraper(ArxivScraper::new()?)
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

        println!("### BEGIN ABSTRACT ###\n");
        println!("{} | ({})", best_rec.text, best_score);
        println!("");
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

