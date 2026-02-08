#![allow(dead_code)] // Methods will be used as features are added

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::{Serialize, de::DeserializeOwned};

use crate::config::Config;
use crate::models::{
    Action, AddComment, AddLabel, ArchiveCard, Board, Card, CreateCard, Label, List,
    UpdateCardDesc, UpdateCardPosition, UpdateListPosition,
};

const BASE_URL: &str = "https://api.trello.com/1";

pub struct TrelloClient {
    client: Client,
    api_key: String,
    api_token: String,
}

#[derive(Debug, Clone)]
pub struct NamedItem {
    pub id: String,
    pub name: String,
    pub context: String,
}

fn looks_like_id(input: &str) -> bool {
    input.len() == 24 && input.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn find_unique_match(items: &[NamedItem], query: &str) -> Result<String> {
    let query_lower = query.to_lowercase();
    let matches: Vec<&NamedItem> = items
        .iter()
        .filter(|item| item.name.to_lowercase().contains(&query_lower))
        .collect();

    match matches.len() {
        0 => anyhow::bail!("No matches found for '{}'", query),
        1 => Ok(matches[0].id.clone()),
        _ => {
            let options = matches
                .iter()
                .map(|item| format!("{} (board: {})", item.name, item.context))
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!(
                "Multiple matches found for '{}': {}. Use -b/--board to disambiguate.",
                query,
                options
            )
        }
    }
}

pub fn compute_position(cards: &[Card], target_pos: usize) -> String {
    if target_pos <= 1 || cards.is_empty() {
        "top".to_string()
    } else if target_pos > cards.len() {
        "bottom".to_string()
    } else {
        let before = cards[target_pos - 2].pos;
        let after = cards[target_pos - 1].pos;
        ((before + after) / 2.0).to_string()
    }
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

    pub fn delete_card(&self, card_id: &str) -> Result<()> {
        let path = format!("/cards/{}", card_id);
        self.delete(&path)
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

    pub fn create_card(&self, body: &CreateCard) -> Result<Card> {
        self.post("/cards", body)
    }

    pub fn resolve_list(&self, list: &str, board_filter: Option<&str>) -> Result<String> {
        if looks_like_id(list) {
            Ok(list.to_string())
        } else {
            self.resolve_list_by_name(list, board_filter)
        }
    }

    pub fn resolve_list_by_name(
        &self,
        list_query: &str,
        board_filter: Option<&str>,
    ) -> Result<String> {
        let boards =
            if let Some(filter) = board_filter {
                if looks_like_id(filter) {
                    vec![self.get_board(filter).with_context(|| {
                        format!("Board ID '{}' not found or inaccessible", filter)
                    })?]
                } else {
                    let all_boards = self.get_member_boards().context("Failed to fetch boards")?;
                    let filter_lower = filter.to_lowercase();
                    let filtered: Vec<_> = all_boards
                        .into_iter()
                        .filter(|b| b.name.to_lowercase().contains(&filter_lower))
                        .collect();

                    if filtered.is_empty() {
                        anyhow::bail!("No boards matching '{}' found", filter);
                    }

                    filtered
                }
            } else {
                self.get_member_boards().context("Failed to fetch boards")?
            };

        if boards.is_empty() {
            anyhow::bail!("No boards found");
        }

        let mut list_items = Vec::new();
        for board in &boards {
            let lists = self
                .get_board_lists(&board.id)
                .with_context(|| format!("Failed to fetch lists for board '{}'", board.name))?;
            for list in lists {
                list_items.push(NamedItem {
                    id: list.id,
                    name: list.name,
                    context: board.name.clone(),
                });
            }
        }

        find_unique_match(&list_items, list_query)
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
                    compute_position(&cards, target_pos)
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

    #[test]
    fn find_unique_match_returns_one_match() {
        let items = vec![
            NamedItem {
                id: "1".to_string(),
                name: "To Do".to_string(),
                context: "Board A".to_string(),
            },
            NamedItem {
                id: "2".to_string(),
                name: "Done".to_string(),
                context: "Board A".to_string(),
            },
        ];

        let id = find_unique_match(&items, "to do").unwrap();
        assert_eq!(id, "1");
    }

    #[test]
    fn find_unique_match_returns_error_on_zero_matches() {
        let items = vec![NamedItem {
            id: "1".to_string(),
            name: "To Do".to_string(),
            context: "Board A".to_string(),
        }];

        let err = find_unique_match(&items, "missing")
            .unwrap_err()
            .to_string();
        assert!(err.contains("No matches found"));
    }

    #[test]
    fn find_unique_match_returns_error_with_context_on_multiple_matches() {
        let items = vec![
            NamedItem {
                id: "1".to_string(),
                name: "To Do".to_string(),
                context: "Board A".to_string(),
            },
            NamedItem {
                id: "2".to_string(),
                name: "To Do".to_string(),
                context: "Board B".to_string(),
            },
        ];

        let err = find_unique_match(&items, "to do").unwrap_err().to_string();
        assert!(err.contains("Multiple matches found"));
        assert!(err.contains("Board A"));
        assert!(err.contains("Board B"));
    }

    #[test]
    fn compute_position_returns_top_for_first_or_less() {
        let cards = vec![
            Card {
                id: "1".to_string(),
                name: "A".to_string(),
                desc: String::new(),
                id_board: "b".to_string(),
                id_list: "l".to_string(),
                id_labels: vec![],
                closed: false,
                pos: 10.0,
            },
            Card {
                id: "2".to_string(),
                name: "B".to_string(),
                desc: String::new(),
                id_board: "b".to_string(),
                id_list: "l".to_string(),
                id_labels: vec![],
                closed: false,
                pos: 20.0,
            },
        ];

        assert_eq!(compute_position(&cards, 1), "top");
    }

    #[test]
    fn compute_position_returns_bottom_beyond_length() {
        let cards = vec![Card {
            id: "1".to_string(),
            name: "A".to_string(),
            desc: String::new(),
            id_board: "b".to_string(),
            id_list: "l".to_string(),
            id_labels: vec![],
            closed: false,
            pos: 10.0,
        }];

        assert_eq!(compute_position(&cards, 2), "bottom");
    }

    #[test]
    fn compute_position_returns_midpoint_for_middle() {
        let cards = vec![
            Card {
                id: "1".to_string(),
                name: "A".to_string(),
                desc: String::new(),
                id_board: "b".to_string(),
                id_list: "l".to_string(),
                id_labels: vec![],
                closed: false,
                pos: 10.0,
            },
            Card {
                id: "2".to_string(),
                name: "B".to_string(),
                desc: String::new(),
                id_board: "b".to_string(),
                id_list: "l".to_string(),
                id_labels: vec![],
                closed: false,
                pos: 20.0,
            },
        ];

        assert_eq!(compute_position(&cards, 2), "15");
    }

    #[test]
    fn compute_position_returns_top_for_empty_list() {
        let cards: Vec<Card> = vec![];
        assert_eq!(compute_position(&cards, 5), "top");
    }
}
