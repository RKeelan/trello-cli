#![allow(dead_code)] // Fields used for deserialization; will be read as features are added

use serde::{Deserialize, Serialize};

/// Represents a Trello card
#[derive(Debug, Deserialize)]
pub struct Card {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub desc: String,
    #[serde(rename = "idBoard")]
    pub id_board: String,
    #[serde(rename = "idLabels", default)]
    pub id_labels: Vec<String>,
    #[serde(default)]
    pub closed: bool,
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
