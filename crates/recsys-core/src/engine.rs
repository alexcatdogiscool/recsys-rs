pub use crate::scoring::{ScoringStrategy, FeatureExtractor};
pub use crate::embedding::EmbeddingProvider;
pub use crate::query_formulator::QueryBuilder;
pub use crate::profile::Profile;
pub use crate::scraper::Scraper;
pub use crate::item::Item;


pub struct RecommendationEngine<F, QB, S, R>
where
    S: ScoringStrategy,
    F: FeatureExtractor<S::Representation>,
    R: Scraper,
    QB: QueryBuilder<R::Representation>,
{
    pub extractor: F,
    pub query_builder: QB,
    pub scorer: S,
    pub scraper: R,
}

impl<F, QB, S, R> RecommendationEngine<F, QB, S, R>
where
    S: ScoringStrategy,
    F: FeatureExtractor::<S::Representation>,
    R: Scraper,
    QB: QueryBuilder<R::Representation>,
{
    pub fn recommend(&self, profile: &Profile, limit_per_source: u32) -> anyhow::Result<Vec<(Item, f32)>> {
        let query = self.query_builder.build_query(&profile.engage_history, self.scraper.query_style_hint())?;
        
        let candidates = self.scraper.scrape(&query, limit_per_source)?;

        let engage_repr = self.extractor.extract_batch(&profile.engage_history).expect("failed to get history repr");
        let ignore_repr = self.extractor.extract_batch(&profile.ignore_history).expect("failed to get history repr");
        let candidate_repr = self.extractor.extract_batch(&candidates).expect("failed to get candidate repr");
        let embedded: Vec<(Item, S::Representation)> = candidates.into_iter().zip(candidate_repr).collect();

        Ok(self.scorer.score(&engage_repr, &ignore_repr, &embedded))
        
    }
}