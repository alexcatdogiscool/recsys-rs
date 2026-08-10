use serde::{Deserialize, Serialize};
use tokio;
use reqwest;

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
    pub dim: u32,
}


impl OllamaApi {

    pub fn new(url: String, dim: u32) -> Self {
        Self { url, dim }
    }

    pub async fn fetch(self, sentences: Vec<String>) -> anyhow::Result<Vec<Vec<f32>>> {
        
        let client = reqwest::Client::new();

        let body = ApiRequest {
            model: "qwen3-embedding:0.6b".to_string(),
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