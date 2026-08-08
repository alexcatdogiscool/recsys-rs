# Project Architecture: General-Purpose Recommendation Library

## Workspace layout

Use a Cargo workspace so the core stays dependency-light and scrapers/LLM backends are optional, swappable crates.

```
recsys/
├── Cargo.toml                 # workspace root
├── crates/
│   ├── recsys-core/           # the library itself — no network, no I/O
│   ├── recsys-scraper-reddit/ # example scraper impl
│   ├── recsys-scraper-wiki/   # example scraper impl
│   ├── recsys-scraper-scholar/
│   ├── recsys-embed-openai/   # EmbeddingProvider impl (API-based)
│   ├── recsys-embed-local/    # EmbeddingProvider impl (candle/ONNX, optional)
│   ├── recsys-llm-openai/     # QueryFormulator impl backed by an LLM
│   └── recsys-cli/            # or recsys-app — your actual "scroll scholar" app
└── examples/
    └── basic_loop.rs
```

Keeping `recsys-core` free of any concrete HTTP/embedding/LLM dependency is the important part — it should only know about traits. Everything else is a plugin crate that depends on core, not the other way around.

## Core traits (`recsys-core`)

```rust
// ---- item.rs ----
use std::collections::HashMap;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Item {
    pub id: String,           // stable unique id (url, or source-prefixed id)
    pub source: String,       // "reddit", "wikipedia", "scholar"
    pub title: String,
    pub text: String,         // canonical text used for embedding (title + body/abstract)
    pub url: Option<String>,
    pub metadata: HashMap<String, Value>, // source-specific extras: upvotes, citations, etc.
}

// ---- profile.rs ----
#[derive(Clone, Debug, Default)]
pub struct Profile {
    pub engage_history: Vec<Item>,
    pub ignore_history: Vec<Item>,
}

// ---- scraper.rs ----
pub trait Scraper {
    fn source_name(&self) -> &str;

    /// Culture hint fed to the QueryFormulator, e.g. "informal, natural questions"
    fn query_style_hint(&self) -> &str { "" }

    fn scrape(&self, keywords: &[String], limit: u32) -> anyhow::Result<Vec<Item>>;
    fn get(&self, id: &str) -> anyhow::Result<Option<Item>>;
}

// ---- query_formulator.rs ----
pub trait QueryFormulator {
    /// Turn recent profile context into per-scraper search keywords.
    fn formulate(&self, topic_context: &str, style_hint: &str) -> anyhow::Result<Vec<String>>;
}

// ---- embedding.rs ----
pub trait EmbeddingProvider {
    fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>>;
    fn embed_batch(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed(t)).collect() // default: naive loop, override for real batching
    }
}

// ---- scoring.rs ----
pub trait ScoringStrategy {
    /// Given profile + embedded candidates, return (item, score) sorted best-first.
    fn score(
        &self,
        profile_engage_vecs: &[Vec<f32>],
        profile_ignore_vecs: &[Vec<f32>],
        candidates: &[(Item, Vec<f32>)],
    ) -> Vec<(Item, f32)>;
}
```

## Default implementations (still in `recsys-core`, but swappable)

```rust
// A default centroid-similarity scorer with an exploration slice.
pub struct CentroidScorer {
    pub ignore_penalty: f32,   // λ
    pub explore_fraction: f32, // e.g. 0.2 = 20% of results biased toward novelty
}

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
```

Ship this as the default so the library is usable out of the box, but nothing stops someone from writing a bandit-based `ScoringStrategy` later — that's the whole point of the trait.

## The orchestrator (also `recsys-core`)

This is the piece that ties everything together and is what your app actually calls.

```rust
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
```

## Data flow (end to end)

```
Profile.engage_history (titles)
        │
        ▼
summarize_recent_engagement()  ──► topic_context (plain text)
        │
        ▼
QueryFormulator.formulate(topic_context, scraper.query_style_hint())
        │  (one LLM call per scraper — text in, text out, no vectors)
        ▼
per-scraper keyword lists
        │
        ▼
Scraper.scrape(keywords, limit)  ──► Vec<Item>   (per source, in parallel)
        │
        ▼
EmbeddingProvider.embed_batch()  ──► all Items into ONE shared vector space
        │                              (profile history embedded the same way)
        ▼
ScoringStrategy.score()  ──► ranked (Item, f32) list
        │
        ▼
Your CLI/UI renders it, records engage/ignore back into Profile
```

## Suggested build order

1. **`recsys-core`** with traits + `CentroidScorer` + orchestrator, using a fake in-memory `Scraper` and a fake `EmbeddingProvider` (return random vectors) — get the plumbing compiling and the loop running end to end with fake data first.
2. **One real `EmbeddingProvider`** (e.g. OpenAI/Voyage embeddings API — simplest possible HTTP call).
3. **One real `Scraper`** (Scholar or Wikipedia — Wikipedia's API is friendliest to start with, no auth).
4. **One real `QueryFormulator`** (LLM call with the style-hint prompt).
5. Wire those three into the orchestrator, confirm you get sane recommendations for a hand-built test `Profile`.
6. Add a second, structurally different `Scraper` (Reddit) to stress-test that the `Item` normalization actually holds up across sources.
7. Only then: build the UI/CLI and start tuning `λ` and `explore_fraction` against real usage.

Steps 1–5 are the whole "does this concept work at all" test — worth doing with the cheapest possible embedding/LLM setup before investing in a nice interface.
