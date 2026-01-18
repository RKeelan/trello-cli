#![allow(dead_code)] // TODO: Remove once client is used

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::{Serialize, de::DeserializeOwned};
use std::env;

const BASE_URL: &str = "https://api.trello.com/1";

pub struct TrelloClient {
    client: Client,
    api_key: String,
    api_token: String,
}

impl TrelloClient {
    pub fn from_env() -> Result<Self> {
        let api_key =
            env::var("TRELLO_API_KEY").context("TRELLO_API_KEY environment variable not set")?;
        let api_token = env::var("TRELLO_API_TOKEN")
            .context("TRELLO_API_TOKEN environment variable not set")?;

        Ok(Self {
            client: Client::new(),
            api_key,
            api_token,
        })
    }

    fn build_url(&self, path: &str) -> String {
        format!("{}{}", BASE_URL, path)
    }

    fn add_auth(&self, url: &str) -> String {
        let separator = if url.contains('?') { '&' } else { '?' };
        format!(
            "{}{}key={}&token={}",
            url, separator, self.api_key, self.api_token
        )
    }

    fn handle_response<T: DeserializeOwned>(response: reqwest::blocking::Response) -> Result<T> {
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("API request failed with status {}: {}", status, body);
        }

        response.json().context("Failed to parse JSON response")
    }

    pub fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.add_auth(&self.build_url(path));
        let response = self
            .client
            .get(&url)
            .send()
            .context("Failed to send GET request")?;

        Self::handle_response(response)
    }

    pub fn put<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T> {
        let url = self.add_auth(&self.build_url(path));
        let response = self
            .client
            .put(&url)
            .json(body)
            .send()
            .context("Failed to send PUT request")?;

        Self::handle_response(response)
    }

    pub fn post<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> Result<T> {
        let url = self.add_auth(&self.build_url(path));
        let response = self
            .client
            .post(&url)
            .json(body)
            .send()
            .context("Failed to send POST request")?;

        Self::handle_response(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_client() -> TrelloClient {
        TrelloClient {
            client: Client::new(),
            api_key: "test_key".to_string(),
            api_token: "test_token".to_string(),
        }
    }

    #[test]
    fn build_url_constructs_correct_path() {
        let client = test_client();
        let url = client.build_url("/cards/123");
        assert_eq!(url, "https://api.trello.com/1/cards/123");
    }

    #[test]
    fn add_auth_adds_query_params() {
        let client = test_client();
        let url = client.add_auth("https://api.trello.com/1/cards/123");
        assert_eq!(
            url,
            "https://api.trello.com/1/cards/123?key=test_key&token=test_token"
        );
    }

    #[test]
    fn add_auth_appends_to_existing_query() {
        let client = test_client();
        let url = client.add_auth("https://api.trello.com/1/cards/123?fields=name");
        assert_eq!(
            url,
            "https://api.trello.com/1/cards/123?fields=name&key=test_key&token=test_token"
        );
    }
}
