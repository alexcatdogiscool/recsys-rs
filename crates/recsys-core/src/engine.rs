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

pub struct Unset;
pub struct EngineBuilder<F = Unset, QB = Unset, S = Unset, R = Unset> {
    pub extractor: F,
    pub query_builder: QB,
    pub scorer: S,
    pub scraper: R,
}

impl EngineBuilder {
    pub fn new() -> Self {
        EngineBuilder { 
            extractor: Unset,
            query_builder: Unset,
            scorer: Unset,
            scraper: Unset
        }
    }
}

impl<F, QB, S, R> EngineBuilder<F, QB, S, R> {
    pub fn scorer<S2>(self, scorer: S2) -> EngineBuilder<F, QB, S2, R> {
        EngineBuilder { extractor: self.extractor, query_builder: self.query_builder, scorer, scraper: self.scraper }
    }

    pub fn extractor<F2>(self, extractor: F2) -> EngineBuilder<F2, QB, S, R> {
        EngineBuilder { extractor, query_builder: self.query_builder, scorer: self.scorer, scraper: self.scraper }
    }

    pub fn scraper<R2>(self, scraper: R2) -> EngineBuilder<F, QB, S, R2> {
        EngineBuilder { extractor: self.extractor, query_builder: self.query_builder, scorer: self.scorer, scraper }
    }

    pub fn query_builder<QB2>(self, query_builder: QB2) -> EngineBuilder<F, QB2, S, R> {
        EngineBuilder { extractor: self.extractor, query_builder, scorer: self.scorer, scraper: self.scraper }
    }
}


impl<F, QB, S, R> EngineBuilder<F, QB, S, R>
where 
    F: FeatureExtractor<S::Representation>,
    QB: QueryBuilder<R::Representation>,
    S: ScoringStrategy,
    R: Scraper,
{
    pub fn build(self) -> RecommendationEngine<F, QB, S, R> {
        RecommendationEngine {
            extractor: self.extractor,
            query_builder: self.query_builder,
            scorer: self.scorer,
            scraper: self.scraper
        }
    }
}