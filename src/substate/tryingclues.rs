use std::hash::Hash;

use hexx::Hex;
use notan::egui::{self, Context};
use strum::IntoEnumIterator;

use crate::model::{
    Animal, Answer, Clue, Map, PlayerList, StructureColor, StructureKind, Terrain, Tile,
};

use super::{placingstructures::PlacingStructures, Common};

#[derive(Debug)]
pub struct TryingClues {
    map: Map,
    /// True: Manually experiment with different possible clues.
    /// False: Deduce clues from given answers.
    manual_clues: bool,
    /// Manually entered clues
    clues: Vec<Clue>,
    highlight: Option<Hex>,
    players: PlayerList,
}

impl From<&PlacingStructures> for TryingClues {
    fn from(value: &PlacingStructures) -> Self {
        let mut s = Self {
            map: Map(value.tiles().to_vec()),
            clues: Vec::new(),
            highlight: Some(Hex::ZERO),
            players: value.players.clone(),
            manual_clues: false,
        };
        s.scan_clues();
        s
    }
}

impl Common for TryingClues {
    fn tiles(&self) -> &[Tile] {
        &self.map.0
    }
    fn tiles_mut(&mut self) -> &mut [Tile] {
        &mut self.map.0
    }

    fn gui(&mut self, ctx: &Context) -> bool {
        let clues_before = self.clues.clone();

        self.gui_for_clues(ctx);
        self.gui_for_questions(ctx);

        if clues_before != self.clues {
            self.scan_clues();
        }

        false
    }

    fn highlight(&self) -> Option<Hex> {
        self.highlight
    }

    fn click(&mut self, hex: Hex) {
        self.highlight = self.map.get(hex).is_some().then_some(hex);
    }

    fn players(&self) -> &PlayerList {
        &self.players
    }
}

impl TryingClues {
    fn gui_for_questions(&mut self, ctx: &Context) {
        egui::Window::new("Answers").show(ctx, |ui| {
            if let Some(selected_tile) = self.highlight.and_then(|hex| self.map.get_mut(hex)) {
                ui.label("Set answers for the selected tile.");
                ui.separator();
                for player in self.players.iter() {
                    let answer = selected_tile.answers.entry(player.id).or_default();
                    ui.horizontal(|ui| {
                        ui.label(&player.name);
                        egui::ComboBox::new(format!("player-answer-{:?}", player.id), "")
                            .selected_text(format!("{answer}"))
                            .show_ui(ui, |ui| {
                                for a in Answer::iter() {
                                    ui.selectable_value(answer, a, format!("{a}"));
                                }
                            });
                    });
                }
            } else {
                ui.label("Select a tile to place anwers.");
            }
        });
    }

    fn gui_for_clues(&mut self, ctx: &Context) {
        // Index of clue to delete
        let mut delete_index = None;

        let remaining_tiles = self.map.0.iter().filter(|t| !t.small).count();

        egui::Window::new("Clues").show(ctx, |ui| {
            ui.label(format!("{remaining_tiles} tiles remain."));

            ui.checkbox(&mut self.manual_clues, "Enter clues manually");

            if self.manual_clues {
                // Edit existing clues
                for (i, clue) in self.clues.iter_mut().enumerate() {
                    ui.separator();

                    match clue {
                        Clue::Terrain(terrain) => {
                            ui.horizontal(|ui| {
                                ui.label("Within one space of");
                                terrain_switcher(format!("terrain-{i}"), ui, terrain);
                            });
                        }
                        Clue::TwoTerrains(a, b) => {
                            ui.horizontal(|ui| {
                                ui.label("On");
                                terrain_switcher(format!("terrain-{i}-a"), ui, a);
                                ui.label("or");
                                terrain_switcher(format!("terrain-{i}-b"), ui, b);
                            });
                        }
                        Clue::EitherAnimal => {
                            ui.label("Within one space of either animal");
                        }
                        Clue::Animal(animal) => {
                            ui.horizontal(|ui| {
                                ui.label("Within two spaces of");
                                egui::ComboBox::new(format!("animal-{i}"), "Territory")
                                    .selected_text(format!("{animal}"))
                                    .show_ui(ui, |ui| {
                                        for a in Animal::iter() {
                                            ui.selectable_value(animal, a, format!("{a}"));
                                        }
                                    });
                            });
                        }
                        Clue::StructureKind(kind) => {
                            ui.horizontal(|ui| {
                                ui.label("Within two spaces of");
                                egui::ComboBox::new(format!("structurekind-{i}"), "")
                                    .selected_text(format!("{kind}"))
                                    .show_ui(ui, |ui| {
                                        for k in StructureKind::iter() {
                                            ui.selectable_value(kind, k, format!("{k}"));
                                        }
                                    });
                            });
                        }
                        Clue::StructureColor(color) => {
                            ui.horizontal(|ui| {
                                ui.label("Within three spaces of");
                                egui::ComboBox::new(format!("structurecolor-{i}"), "structure")
                                    .selected_text(format!("{color}"))
                                    .show_ui(ui, |ui| {
                                        for c in StructureColor::iter() {
                                            ui.selectable_value(color, c, format!("{c}"));
                                        }
                                    });
                            });
                        }
                    }

                    if ui.button("Delete").clicked() {
                        delete_index = Some(i);
                    }
                }

                if let Some(delete) = delete_index {
                    self.clues.remove(delete);
                }

                ui.separator();

                // Add a new clue
                egui::ComboBox::new("combobox-new-clue", "")
                    .selected_text("Add clue")
                    .show_ui(ui, |ui| {
                        if ui.button("Within one space of terrain").clicked() {
                            self.clues.push(Clue::Terrain(Terrain::Desert));
                        }
                        if ui.button("One of two terrains").clicked() {
                            self.clues
                                .push(Clue::TwoTerrains(Terrain::Desert, Terrain::Forest));
                        }
                        if ui.button("Within one space of either animal").clicked() {
                            self.clues.push(Clue::EitherAnimal);
                        }
                        if ui.button("Within two spaces of animal").clicked() {
                            self.clues.push(Clue::Animal(Animal::Bear));
                        }
                        if ui.button("Within two spaces of structure type").clicked() {
                            self.clues.push(Clue::StructureKind(StructureKind::Shack));
                        }
                        if ui
                            .button("Within three spaces of structure color")
                            .clicked()
                        {
                            self.clues.push(Clue::StructureColor(StructureColor::Black));
                        }
                    });
            } else {
                // Deduce clues for players
                for player in self.players.iter() {
                    ui.separator();
                    // TODO Cache clues and only recompute when the answers change.
                    let clues = self.map.clues_for_player(player.id);
                    egui::CollapsingHeader::new(format!(
                        "{} possible clues for {}",
                        clues.len(),
                        player.name
                    ))
                    .id_source(player.id)
                    .show(ui, |ui| {
                        for clue in clues {
                            ui.label(format!("{clue}"));
                        }
                    });
                }
            }
        });
    }

    /// Go through all tiles and see if any clue applies to them.
    /// If no clue applies to them, they are drawn as small.
    fn scan_clues(&mut self) {
        // Set tile to be big. Should any clue fail, then it will be small.
        for tile in &mut self.map.0 {
            tile.small = false;
        }

        for &clue in &self.clues {
            for i in 0..self.map.0.len() {
                let position = self.map.0[i].position;
                let found = self.map.clue_applies(clue, position);
                if !found {
                    self.map.0[i].small = true;
                }
            }
        }
    }
}

// Dropdown for switching terrain types.
fn terrain_switcher(id: impl Hash, ui: &mut egui::Ui, terrain: &mut Terrain) {
    egui::ComboBox::new(id, "")
        .selected_text(format!("{terrain}"))
        .show_ui(ui, |ui| {
            for t in Terrain::iter() {
                ui.selectable_value(terrain, t, format!("{t}"));
            }
        });
}
