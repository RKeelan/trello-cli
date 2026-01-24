#![allow(dead_code)] // Methods will be used as features are added

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::{Serialize, de::DeserializeOwned};

use crate::config::Config;
use crate::models::{
    Action, AddComment, AddLabel, ArchiveCard, Board, Card, Label, List, UpdateCardDesc,
    UpdateCardPosition, UpdateListPosition,
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

    /// Apply a label by name to a card using pre-fetched card and board labels.
    pub fn apply_label_by_name(
        &self,
        card: &Card,
        labels: &[Label],
        label_name: &str,
    ) -> Result<()> {
        let label = labels
            .iter()
            .find(|l| l.name.eq_ignore_ascii_case(label_name))
            .ok_or_else(|| anyhow::anyhow!("Label '{}' not found on board", label_name))?;

        if !card.id_labels.contains(&label.id) {
            self.add_label_to_card(&card.id, &label.id)?;
        }

        Ok(())
    }

    pub fn remove_label_from_card(&self, card_id: &str, label_id: &str) -> Result<()> {
        let path = format!("/cards/{}/idLabels/{}", card_id, label_id);
        self.delete(&path)
    }

    /// Remove a label by name from a card using pre-fetched card and board labels.
    pub fn remove_label_by_name(
        &self,
        card: &Card,
        labels: &[Label],
        label_name: &str,
    ) -> Result<()> {
        let label = labels
            .iter()
            .find(|l| l.name.eq_ignore_ascii_case(label_name))
            .ok_or_else(|| anyhow::anyhow!("Label '{}' not found on board", label_name))?;

        if card.id_labels.contains(&label.id) {
            self.remove_label_from_card(&card.id, &label.id)?;
        }

        Ok(())
    }

    /// Archive a card using a pre-fetched Card.
    pub fn archive_card(&self, card: &Card) -> Result<()> {
        if !card.closed {
            let path = format!("/cards/{}", card.id);
            let body = ArchiveCard { closed: true };
            self.put::<Card, _>(&path, &body)?;
        }
        Ok(())
    }

    /// Restore (unarchive) a card using a pre-fetched Card.
    pub fn restore_card(&self, card: &Card) -> Result<()> {
        if card.closed {
            let path = format!("/cards/{}", card.id);
            let body = ArchiveCard { closed: false };
            self.put::<Card, _>(&path, &body)?;
        }
        Ok(())
    }

    pub fn add_comment_to_card(&self, card_id: &str, text: &str) -> Result<Action> {
        let path = format!("/cards/{}/actions/comments", card_id);
        let body = AddComment {
            text: text.to_string(),
        };
        self.post(&path, &body)
    }

    pub fn get_list_cards(&self, list_id: &str) -> Result<Vec<Card>> {
        let path = format!("/lists/{}/cards", list_id);
        self.get(&path)
    }

    pub fn get_card_comments(&self, card_id: &str) -> Result<Vec<Action>> {
        let mut all_comments = Vec::new();
        let limit = 1000;
        let mut before: Option<String> = None;

        loop {
            let path = match &before {
                Some(id) => format!(
                    "/cards/{}/actions?filter=commentCard&limit={}&before={}",
                    card_id, limit, id
                ),
                None => format!(
                    "/cards/{}/actions?filter=commentCard&limit={}",
                    card_id, limit
                ),
            };

            let batch: Vec<Action> = self.get(&path)?;
            let batch_size = batch.len();
            if batch_size == 0 {
                break;
            }

            before = batch.last().map(|a| a.id.clone());
            all_comments.extend(batch);

            // If we got fewer than limit, we've reached the end
            if batch_size < limit {
                break;
            }
        }

        Ok(all_comments)
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
    use crate::config::CredentialSource;
    use std::collections::HashMap;

    fn test_client() -> TrelloClient {
        TrelloClient {
            client: Client::new(),
            api_key: "test_key".to_string(),
            api_token: "test_token".to_string(),
        }
    }

    struct MockSource(HashMap<String, String>);

    impl MockSource {
        fn new() -> Self {
            MockSource(HashMap::new())
        }

        fn with(vars: &[(&str, &str)]) -> Self {
            let map = vars
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            MockSource(map)
        }
    }

    impl CredentialSource for MockSource {
        fn get(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
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

        // Use empty mock source to force file-based loading
        let source = MockSource::new();
        let config = Config::load_from_source(&source, config_path).unwrap();
        let client = TrelloClient::new(&config);

        assert_eq!(client.api_key, "config_key");
        assert_eq!(client.api_token, "config_token");
    }

    #[test]
    fn client_new_from_env_source() {
        use tempfile::TempDir;

        let source = MockSource::with(&[
            ("TRELLO_API_KEY", "env_key"),
            ("TRELLO_API_TOKEN", "env_token"),
        ]);

        // Config path doesn't need to exist when env vars are set
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let config = Config::load_from_source(&source, config_path).unwrap();
        let client = TrelloClient::new(&config);

        assert_eq!(client.api_key, "env_key");
        assert_eq!(client.api_token, "env_token");
    }
}
