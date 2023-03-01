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
    pub players: Vec<String>,
    map_ready: bool,
    players_ready: bool,
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
            map_ready: false,
            players_ready: false,
            players: Vec::new(),
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

        if !self.map_ready {
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
                        self.map_ready = true;
                    }
                } else {
                    ui.label("Select every piece once to continue");
                }
            });
        }

        if !self.players_ready {
            egui::Window::new("Players").show(ctx, |ui| {
                let mut remove = None;
                for (i, player) in self.players.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(player);
                        if ui.button("X").clicked() {
                            remove = Some(i);
                        }
                    });
                }

                if let Some(i) = remove {
                    self.players.remove(i);
                }

                ui.horizontal(|ui| {
                    if ui.button("Add").clicked() {
                        self.players.push("Some Player".to_owned());
                    }

                    if self.players.len() >= 3 && self.players.len() <= 5 {
                        if ui.button("Ready").clicked() {
                            self.players_ready = true;
                        }
                    } else {
                        ui.label("Add 3 to 5 players to continue");
                    }
                });
            });
        }

        if selected_pieces_before != self.selected_pieces {
            self.rebuild_tiles();
        }

        self.map_ready && self.players_ready
    }

    fn highlight(&self) -> Option<Hex> {
        None
    }

    fn click(&mut self, _hex: Hex) {}
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
