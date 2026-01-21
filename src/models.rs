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

/// Request body for updating a card's position
#[derive(Debug, Serialize)]
pub struct UpdateCardPosition {
    pub pos: String,
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
