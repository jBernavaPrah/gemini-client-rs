use super::{API_BASE, request, response};
use derive_new::new;
use derive_setters::Setters;
use serde_json;
use thiserror::Error;
use tokio_stream::StreamExt;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    ApiError(String),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, new, Setters)]
#[setters(prefix = "with_", into, strip_option)]
pub struct Client {
    #[setters(skip)]
    #[new(into)]
    api_key: String,
    #[setters(skip)]
    #[new(into)]
    model: String,
    #[new(value = "API_BASE.to_string()")]
    api_base: String,
    #[new(value = "reqwest::Client::new()")]
    client: reqwest::Client,
}

impl Client {
    pub async fn generate_content(
        &self,
        request: request::Request,
    ) -> Result<response::Response, Error> {
        let url = format!(
            "{api_base}/{model}:generateContent?key={api_key}",
            api_base = self.api_base,
            model = self.model,
            api_key = self.api_key,
        );

        let response = self
            .client
            .post(&url)
            .header(reqwest::header::USER_AGENT, env!("CARGO_CRATE_NAME"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|e| format!("Failed to read error body: {}", e));
            return Err(Error::ApiError(error_body));
        }

        Ok(response.json().await?)
    }

    pub async fn stream_content(
        &self,
        request: request::Request,
    ) -> Result<impl tokio_stream::Stream<Item = Result<response::Response, Error>>, Error> {
        let url = format!(
            "{api_base}/{model}:streamGenerateContent?alt=sse&key={api_key}",
            api_base = self.api_base,
            model = self.model,
            api_key = self.api_key,
        );

        let response = self
            .client
            .post(&url)
            .header(reqwest::header::USER_AGENT, env!("CARGO_CRATE_NAME"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|e| format!("Failed to read error body: {}", e));
            return Err(Error::ApiError(error_body));
        }

        Ok(response.bytes_stream().map(|x| {
            let bytes = x.map_err(Error::from)?;
            let text = String::from_utf8_lossy(&bytes);
            text.strip_prefix("data: ")
                .ok_or_else(|| {
                    Error::ApiError(format!(
                        "Invalid SSE data format, missing 'data: ' prefix. Full text: '{}'",
                        text
                    ))
                })
                .and_then(|s| serde_json::from_str::<response::Response>(s).map_err(Error::from))
        }))
    }
}
