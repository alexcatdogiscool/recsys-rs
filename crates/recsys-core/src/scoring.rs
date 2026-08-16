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
    pub ignore_penalty: f32,
    pub explore_fraction: f32,
}


impl ScoringStrategy for CentroidScorer {
    type Representation = Vec<f32>;
    fn score(&self, engage: &[Vec<f32>], ignore: &[Vec<f32>], candidates: &[(Item, Vec<f32>)]) -> Vec<(Item, f32)> {
        let engage_centroid = centroid(engage);
        let ignore_centroid = centroid(ignore);

        let mut scored: Vec<(Item, f32)> = candidates.iter().map(|(item, vec)| {
            let engage_sim = match &engage_centroid {
                Some(c) => cosine(vec, c),
                None => 0.0,
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


