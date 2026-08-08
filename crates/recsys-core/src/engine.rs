pub use crate::scoring::ScoringStrategy;
pub use crate::embedding::EmbeddingProvider;
pub use crate::query_formulator::QueryFormulator;
pub use crate::profile::Profile;
pub use crate::scraper::Scraper;
pub use crate::item::Item;


pub struct RecommendationEngine<E: EmbeddingProvider, Q: QueryFormulator, S: ScoringStrategy> {
    pub embedder: E,
    pub formulator: Q,
    pub scorer: S,
    pub scrapers: Vec<Box<dyn Scraper>>,
}

impl<E: EmbeddingProvider, Q: QueryFormulator, S: ScoringStrategy> RecommendationEngine<E, Q, S> {
    pub fn recommend(&self, profile: &Profile, limit_per_source: u32) -> anyhow::Result<Vec<(Item, f32)>> {
        // 1. Build topic context from recent engagement (plain text, no embeddings)
        let topic_context = summarize_recent_engagement(&profile.engage_history);

        // 2. Per-scraper query formulation + scrape
        let mut candidates = Vec::new();
        for scraper in &self.scrapers {
            let keywords = self.formulator.formulate(&topic_context, scraper.query_style_hint())?;
            candidates.extend(scraper.scrape(&keywords, limit_per_source)?);
        }

        // 3. Embed everything in one shared space
        let engage_vecs = self.embedder.embed_batch(
            &profile.engage_history.iter().map(|i| i.text.clone()).collect::<Vec<_>>()
        )?;
        let ignore_vecs = self.embedder.embed_batch(
            &profile.ignore_history.iter().map(|i| i.text.clone()).collect::<Vec<_>>()
        )?;
        let candidate_texts: Vec<String> = candidates.iter().map(|i| i.text.clone()).collect();
        let candidate_vecs = self.embedder.embed_batch(&candidate_texts)?;
        let embedded_candidates: Vec<(Item, Vec<f32>)> =
            candidates.into_iter().zip(candidate_vecs).collect();

        // 4. Score
        Ok(self.scorer.score(&engage_vecs, &ignore_vecs, &embedded_candidates))
    }
}

fn summarize_recent_engagement(history: &[Item]) -> String {
    // simplest version: just join recent titles. Can upgrade to TF-IDF/RAKE later.
    history.iter().rev().take(20).map(|i| i.title.clone())
        .collect::<Vec<_>>().join("; ")
}