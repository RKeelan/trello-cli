#![allow(dead_code)] // Methods will be used as features are added

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::{Serialize, de::DeserializeOwned};
use std::env;

use crate::models::{AddLabel, Card, Label, UpdateCardDesc};

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

    pub fn delete(&self, path: &str) -> Result<()> {
        let url = self.add_auth(&self.build_url(path));
        let response = self
            .client
            .delete(&url)
            .send()
            .context("Failed to send DELETE request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("API request failed with status {}: {}", status, body);
        }

        Ok(())
    }

    // Card operations

    pub fn update_card_description(&self, card_id: &str, description: &str) -> Result<Card> {
        let path = format!("/cards/{}", card_id);
        let body = UpdateCardDesc {
            desc: description.to_string(),
        };
        self.put(&path, &body)
    }

    pub fn get_card(&self, card_id: &str) -> Result<Card> {
        let path = format!("/cards/{}", card_id);
        self.get(&path)
    }

    pub fn get_board_labels(&self, board_id: &str) -> Result<Vec<Label>> {
        let path = format!("/boards/{}/labels", board_id);
        self.get(&path)
    }

    pub fn add_label_to_card(&self, card_id: &str, label_id: &str) -> Result<Vec<String>> {
        let path = format!("/cards/{}/idLabels", card_id);
        let body = AddLabel {
            value: label_id.to_string(),
        };
        self.post(&path, &body)
    }

    /// Apply a label by name to a card.
    /// Fetches the card to get its board, then finds the label by name.
    /// Returns the card name for confirmation messages.
    pub fn apply_label_by_name(&self, card_id: &str, label_name: &str) -> Result<String> {
        let card = self.get_card(card_id)?;
        let labels = self.get_board_labels(&card.id_board)?;

        let label = labels
            .iter()
            .find(|l| l.name.eq_ignore_ascii_case(label_name))
            .ok_or_else(|| anyhow::anyhow!("Label '{}' not found on board", label_name))?;

        // Check if label is already applied
        if !card.id_labels.contains(&label.id) {
            self.add_label_to_card(card_id, &label.id)?;
        }

        Ok(card.name)
    }

    pub fn remove_label_from_card(&self, card_id: &str, label_id: &str) -> Result<()> {
        let path = format!("/cards/{}/idLabels/{}", card_id, label_id);
        self.delete(&path)
    }

    /// Remove a label by name from a card.
    /// Fetches the card to get its board, then finds the label by name.
    /// Returns the card name for confirmation messages.
    pub fn remove_label_by_name(&self, card_id: &str, label_name: &str) -> Result<String> {
        let card = self.get_card(card_id)?;
        let labels = self.get_board_labels(&card.id_board)?;

        let label = labels
            .iter()
            .find(|l| l.name.eq_ignore_ascii_case(label_name))
            .ok_or_else(|| anyhow::anyhow!("Label '{}' not found on board", label_name))?;

        // Only remove if label is applied
        if card.id_labels.contains(&label.id) {
            self.remove_label_from_card(card_id, &label.id)?;
        }

        Ok(card.name)
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
