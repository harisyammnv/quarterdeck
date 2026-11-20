//! Board commands: the board and the open decisions, projected from the fleet snapshot.

use quarterdeck_core::board::{self, BoardView};
use serde::Serialize;
use specta::Type;

use crate::state::AppState;

/// A new board, pushed only when it differs from the last one sent.
#[derive(Clone, Serialize, Type, tauri_specta::Event)]
pub struct BoardUpdated(pub BoardView);

/// The current board and open decisions, or `None` when no home is configured.
#[tauri::command]
#[specta::specta]
pub fn board_view(state: tauri::State<'_, AppState>) -> Option<BoardView> {
    state.with_engine(|e| board::view(&e.state())).ok()
}
