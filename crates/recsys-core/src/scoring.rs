pub use crate::Item;

pub trait ScoringStrategy {
    type Representation;
    
    fn score(
        &self,
        engage: &[Self::Representation],
        ignore: &[Self::Representation],
        candidates: &[(Item, Self::Representation)],
    ) -> Vec<(Item, f32)>;
}

pub trait FeatureExtractor<Repr> {
    fn extract(&self, item: &Item) -> anyhow::Result<Repr>;

    fn extract_batch(&self, items: &[Item]) -> anyhow::Result<Vec<Repr>> {
        items.iter().map(|item| self.extract(item)).collect()
    }
}


pub struct CentroidScorer {
    pub ignore_penalty: f32,   // λ
    pub explore_fraction: f32, // e.g. 0.2 = 20% of results biased toward novelty
}

/*
impl ScoringStrategy for CentroidScorer {
    fn score(&self, engage: &[Vec<f32>], ignore: &[Vec<f32>], candidates: &[(Item, Vec<f32>)]) -> Vec<(Item, f32)> {
        let engage_centroid = centroid(engage);
        let ignore_centroid = centroid(ignore);

        let mut scored: Vec<(Item, f32)> = candidates.iter().map(|(item, vec)| {
            let s = cosine(vec, &engage_centroid) - self.ignore_penalty * cosine(vec, &ignore_centroid);
            (item.clone(), s)
        }).collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        // TODO: reserve explore_fraction of slots for items near cluster edge, not center
        scored
    }
}
*/


