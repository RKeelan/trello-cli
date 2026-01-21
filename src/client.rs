#![allow(dead_code)] // Methods will be used as features are added

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::{Serialize, de::DeserializeOwned};

use crate::config::Config;
use crate::models::{
    AddLabel, ArchiveCard, Board, Card, Label, List, UpdateCardDesc, UpdateCardPosition,
    UpdateListPosition,
};

const BASE_URL: &str = "https://api.trello.com/1";

pub struct TrelloClient {
    client: Client,
    api_key: String,
    api_token: String,
}

impl TrelloClient {
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            api_key: config.api_key().to_string(),
            api_token: config.api_token().to_string(),
        }
    }

    pub fn from_env() -> Result<Self> {
        let config = Config::load()?;
        Ok(Self::new(&config))
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

    pub fn archive_card(&self, card_id: &str) -> Result<String> {
        let card = self.get_card(card_id)?;
        if !card.closed {
            let path = format!("/cards/{}", card_id);
            let body = ArchiveCard { closed: true };
            self.put::<Card, _>(&path, &body)?;
        }
        Ok(card.name)
    }

    pub fn get_list_cards(&self, list_id: &str) -> Result<Vec<Card>> {
        let path = format!("/lists/{}/cards", list_id);
        self.get(&path)
    }

    pub fn move_card(&self, card_id: &str, position: &str) -> Result<Card> {
        let pos_value = match position {
            "top" | "bottom" => position.to_string(),
            _ => {
                if let Ok(target_pos) = position.parse::<usize>() {
                    let card = self.get_card(card_id)?;
                    let mut cards: Vec<Card> = self
                        .get_list_cards(&card.id_list)?
                        .into_iter()
                        .filter(|c| c.id != card.id)
                        .collect();
                    cards.sort_by(|a, b| a.pos.partial_cmp(&b.pos).unwrap());

                    if target_pos <= 1 || cards.is_empty() {
                        "top".to_string()
                    } else if target_pos > cards.len() {
                        "bottom".to_string()
                    } else {
                        // Position between cards[target_pos-2] and cards[target_pos-1]
                        let before = cards[target_pos - 2].pos;
                        let after = cards[target_pos - 1].pos;
                        ((before + after) / 2.0).to_string()
                    }
                } else {
                    position.to_string()
                }
            }
        };

        let path = format!("/cards/{}", card_id);
        let body = UpdateCardPosition { pos: pos_value };
        self.put(&path, &body)
    }

    // List operations

    pub fn get_list(&self, list_id: &str) -> Result<List> {
        let path = format!("/lists/{}", list_id);
        self.get(&path)
    }

    pub fn get_board_lists(&self, board_id: &str) -> Result<Vec<List>> {
        let path = format!("/boards/{}/lists", board_id);
        self.get(&path)
    }

    pub fn move_list(&self, list_id: &str, position: &str) -> Result<List> {
        let pos_value = match position {
            "top" | "bottom" => position.to_string(),
            _ => {
                if let Ok(target_pos) = position.parse::<usize>() {
                    let list = self.get_list(list_id)?;
                    let mut lists: Vec<List> = self
                        .get_board_lists(&list.id_board)?
                        .into_iter()
                        .filter(|l| l.id != list.id)
                        .collect();
                    lists.sort_by(|a, b| a.pos.partial_cmp(&b.pos).unwrap());

                    if target_pos <= 1 || lists.is_empty() {
                        "top".to_string()
                    } else if target_pos > lists.len() {
                        "bottom".to_string()
                    } else {
                        // Position between lists[target_pos-2] and lists[target_pos-1]
                        let before = lists[target_pos - 2].pos;
                        let after = lists[target_pos - 1].pos;
                        ((before + after) / 2.0).to_string()
                    }
                } else {
                    position.to_string()
                }
            }
        };

        let path = format!("/lists/{}", list_id);
        let body = UpdateListPosition { pos: pos_value };
        self.put(&path, &body)
    }

    // Board operations

    pub fn get_member_boards(&self) -> Result<Vec<Board>> {
        self.get("/members/me/boards?filter=open")
    }

    pub fn get_board(&self, board_id: &str) -> Result<Board> {
        let path = format!("/boards/{}", board_id);
        self.get(&path)
    }

    pub fn get_board_cards(&self, board_id: &str) -> Result<Vec<Card>> {
        let path = format!("/boards/{}/cards", board_id);
        self.get(&path)
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

    #[test]
    fn client_new_from_config() {
        use std::fs;
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        writeln!(file, "api_key = \"config_key\"").unwrap();
        writeln!(file, "api_token = \"config_token\"").unwrap();

        // Clear env vars to force file-based loading
        // SAFETY: Tests run with --test-threads=1
        unsafe {
            std::env::remove_var("TRELLO_API_KEY");
            std::env::remove_var("TRELLO_API_TOKEN");
        }

        let config = Config::load_from_path(config_path).unwrap();
        let client = TrelloClient::new(&config);

        assert_eq!(client.api_key, "config_key");
        assert_eq!(client.api_token, "config_token");
    }

    #[test]
    fn from_env_delegates_to_config_load() {
        // Set env vars so Config::load() succeeds
        // SAFETY: Tests run with --test-threads=1
        unsafe {
            std::env::set_var("TRELLO_API_KEY", "env_key");
            std::env::set_var("TRELLO_API_TOKEN", "env_token");
        }

        let client = TrelloClient::from_env().unwrap();

        assert_eq!(client.api_key, "env_key");
        assert_eq!(client.api_token, "env_token");

        // Clean up
        unsafe {
            std::env::remove_var("TRELLO_API_KEY");
            std::env::remove_var("TRELLO_API_TOKEN");
        }
    }
}
