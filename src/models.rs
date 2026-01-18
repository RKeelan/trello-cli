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
    #[serde(default)]
    pub closed: bool,
}

/// Request body for updating a card's description
#[derive(Debug, Serialize)]
pub struct UpdateCardDesc {
    pub desc: String,
}
