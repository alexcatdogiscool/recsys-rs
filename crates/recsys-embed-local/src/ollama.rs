use serde::{Deserialize, Serialize};
use tokio;
use reqwest;
use recsys_core::{
    FeatureExtractor,
    Item,
};

#[derive(Serialize, Deserialize)]
struct ApiRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApiResponse {
    model: String,
    embeddings: Vec<Vec<f32>>,
    total_duration: u64,
    load_duration: u64,
    prompt_eval_count: u32,
}

#[derive(Clone)]
pub struct OllamaApi {
    pub url: String,
    pub model_name: String,
    pub dim: u32,
    pub context_window_size: u32,
}


impl OllamaApi {

    pub fn new(url: String, model_name: String, dim: u32, context_window_size: u32) -> Self {
        Self { url, model_name, dim, context_window_size }
    }

    pub async fn fetch(self, sentences: Vec<String>) -> anyhow::Result<Vec<Vec<f32>>> {
        
        let client = reqwest::Client::new();

        let body = ApiRequest {
            model: self.model_name,
            input: sentences,
        };

        let response: ApiResponse = client
            .post(&self.url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(response.embeddings)
    }
}

impl FeatureExtractor<Vec<f32>> for OllamaApi {
fn extract(&self, item: &Item) -> anyhow::Result<Vec<f32>> {
    let rt = tokio::runtime::Runtime::new()?;
    let mut result = rt.block_on(self.clone().fetch(vec![item.text.clone()]))?;
    result.pop().ok_or_else(|| anyhow::anyhow!("empty embedding response"))
}
}