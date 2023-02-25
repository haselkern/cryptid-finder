use std::collections::HashSet;

use hexx::{Hex, OffsetHexMode};
use notan::egui;
use strum::IntoEnumIterator;

use crate::model::{Piece, PieceChoice, Tile};

use super::Common;

/// A sub state for functionality for building a map.
#[derive(Debug)]
pub struct BuildingMap {
    selected_pieces: [PieceChoice; 6],
    tiles: Vec<Tile>,
}

impl Default for BuildingMap {
    fn default() -> Self {
        let mut s = Self {
            selected_pieces: Piece::iter()
                .map(Into::into)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            tiles: Vec::new(),
        };

        s.rebuild_tiles();
        s
    }
}

impl Common for BuildingMap {
    fn tiles(&self) -> &[Tile] {
        &self.tiles
    }
    fn tiles_mut(&mut self) -> &mut [Tile] {
        &mut self.tiles
    }

    fn gui(&mut self, ctx: &egui::Context) -> bool {
        let selected_pieces_before = self.selected_pieces;
        let mut next_state = false;

        egui::Window::new("Map Setup").show(ctx, |ui| {
            egui::Grid::new("map-setup-grid").show(ui, |ui| {
                for i in 0..6 {
                    egui::ComboBox::new(format!("map-setup-choice-{i}"), "")
                        .selected_text(format!("{}", self.selected_pieces[i]))
                        .show_ui(ui, |ui| {
                            for piece in Piece::iter() {
                                for rotated in [false, true] {
                                    let choice = PieceChoice { piece, rotated };
                                    ui.selectable_value(
                                        &mut self.selected_pieces[i],
                                        choice,
                                        format!("{choice}"),
                                    );
                                }
                            }
                        });

                    if i % 2 > 0 {
                        ui.end_row();
                    }
                }
            });

            if are_selected_pieces_valid(&self.selected_pieces) {
                if ui.button("Ready").clicked() {
                    next_state = true;
                }
            } else {
                ui.label("Select every piece once to continue");
            }
        });

        if selected_pieces_before != self.selected_pieces {
            self.rebuild_tiles();
        }

        next_state
    }
}

impl BuildingMap {
    /// Update tiles after user changed something
    fn rebuild_tiles(&mut self) {
        let offsets = [
            Hex::ZERO,
            Hex::from_offset_coordinates([6, 0], OffsetHexMode::OddColumns),
            Hex::from_offset_coordinates([0, 3], OffsetHexMode::OddColumns),
            Hex::from_offset_coordinates([6, 3], OffsetHexMode::OddColumns),
            Hex::from_offset_coordinates([0, 6], OffsetHexMode::OddColumns),
            Hex::from_offset_coordinates([6, 6], OffsetHexMode::OddColumns),
        ];
        self.tiles = offsets
            .iter()
            .zip(self.selected_pieces.iter())
            .flat_map(|(&offset, piece)| {
                let mut tiles = piece.piece.parse();
                if piece.rotated {
                    tiles.rotate();
                }
                tiles.translate(offset);
                tiles.0
            })
            .collect();
    }
}

/// Returns true if six different [Piece]s were selected.
fn are_selected_pieces_valid(pieces: &[PieceChoice]) -> bool {
    let pieces: HashSet<Piece> = pieces.iter().map(|choice| choice.piece).collect();
    pieces.len() == 6
}
