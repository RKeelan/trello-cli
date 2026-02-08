#![allow(dead_code)] // Fields used for deserialization; will be read as features are added

use serde::{Deserialize, Serialize};

/// Represents a Trello board
#[derive(Debug, Deserialize, Clone)]
pub struct Board {
    pub id: String,
    pub name: String,
}

/// Represents a Trello card
#[derive(Debug, Deserialize, Clone)]
pub struct Card {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub desc: String,
    #[serde(rename = "idBoard")]
    pub id_board: String,
    #[serde(rename = "idList")]
    pub id_list: String,
    #[serde(rename = "idLabels", default)]
    pub id_labels: Vec<String>,
    #[serde(default)]
    pub closed: bool,
    pub pos: f64,
}

/// Request body for updating a card's description
#[derive(Debug, Serialize)]
pub struct UpdateCardDesc {
    pub desc: String,
}

/// Represents a Trello label
#[derive(Debug, Deserialize)]
pub struct Label {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
}

/// Request body for adding a label to a card
#[derive(Debug, Serialize)]
pub struct AddLabel {
    pub value: String,
}

/// Request body for archiving a card
#[derive(Debug, Serialize)]
pub struct ArchiveCard {
    pub closed: bool,
}

/// Request body for adding a comment to a card
#[derive(Debug, Serialize)]
pub struct AddComment {
    pub text: String,
}

/// Request body for updating a card's position
#[derive(Debug, Serialize)]
pub struct UpdateCardPosition {
    pub pos: String,
}

/// Request body for creating a card
#[derive(Debug, Serialize)]
pub struct CreateCard {
    pub name: String,
    pub pos: String,
    #[serde(rename = "idList")]
    pub id_list: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
}

/// Represents a Trello list
#[derive(Debug, Deserialize, Clone)]
pub struct List {
    pub id: String,
    pub name: String,
    #[serde(rename = "idBoard")]
    pub id_board: String,
    pub pos: f64,
}

/// Request body for updating a list's position
#[derive(Debug, Serialize)]
pub struct UpdateListPosition {
    pub pos: String,
}

/// Represents a Trello action (used for comments)
#[derive(Debug, Deserialize)]
pub struct Action {
    pub id: String,
    #[serde(rename = "type")]
    pub action_type: String,
    pub date: String, // ISO 8601 timestamp, e.g., "2020-03-09T19:41:51.396Z"
    pub data: ActionData,
    #[serde(rename = "memberCreator")]
    pub member_creator: ActionMember, // Always present for commentCard actions
}

#[derive(Debug, Deserialize)]
pub struct ActionData {
    #[serde(default)]
    pub text: String, // Comment text; empty string if not present
}

#[derive(Debug, Deserialize)]
pub struct ActionMember {
    #[serde(rename = "fullName")]
    pub full_name: Option<String>,
    pub username: String,
}

#[cfg(test)]
mod tests {
    use super::CreateCard;

    #[test]
    fn create_card_serializes_with_description() {
        let body = CreateCard {
            name: "New card".to_string(),
            pos: "top".to_string(),
            id_list: "507f1f77bcf86cd799439011".to_string(),
            desc: Some("Some description".to_string()),
        };

        let value = serde_json::to_value(body).unwrap();
        assert_eq!(value["name"], "New card");
        assert_eq!(value["pos"], "top");
        assert_eq!(value["idList"], "507f1f77bcf86cd799439011");
        assert_eq!(value["desc"], "Some description");
    }

    #[test]
    fn create_card_serializes_without_description() {
        let body = CreateCard {
            name: "New card".to_string(),
            pos: "bottom".to_string(),
            id_list: "507f1f77bcf86cd799439011".to_string(),
            desc: None,
        };

        let value = serde_json::to_value(body).unwrap();
        assert_eq!(value["name"], "New card");
        assert_eq!(value["pos"], "bottom");
        assert_eq!(value["idList"], "507f1f77bcf86cd799439011");
        assert!(value.get("desc").is_none());
    }
}
