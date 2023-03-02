use std::{collections::HashMap, hash::Hash};

use hexx::Hex;
use notan::egui::{self, Context};
use strum::IntoEnumIterator;

use crate::model::{
    Animal, Answer, Clue, Map, PlayerID, PlayerList, StructureColor, StructureKind, Terrain, Tile,
};

use super::{placingstructures::PlacingStructures, Common};

#[derive(Debug)]
pub struct TryingClues {
    map: Map,
    /// Manually entered clues
    clues: HashMap<PlayerID, Clue>,
    known_clues: HashMap<PlayerID, bool>,
    highlight: Option<Hex>,
    players: PlayerList,
}

impl From<&PlacingStructures> for TryingClues {
    fn from(value: &PlacingStructures) -> Self {
        let mut s = Self {
            map: Map(value.tiles().to_vec()),
            highlight: Some(Hex::ZERO),
            players: value.players.clone(),
            clues: HashMap::new(),
            known_clues: HashMap::new(),
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
        let known_clues_before = self.known_clues.clone();
        let tiles_before = self.tiles().to_vec();

        self.gui_for_clues(ctx);
        self.gui_for_questions(ctx);

        if clues_before != self.clues
            || known_clues_before != self.known_clues
            || !itertools::equal(&tiles_before, self.tiles())
        {
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
        let remaining_tiles = self.map.0.iter().filter(|t| !t.small).count();

        egui::Window::new("Clues").show(ctx, |ui| {
            ui.label(format!("{remaining_tiles} tiles remain."));

            for player in self.players.iter().map(|p| p.id) {
                ui.separator();
                // Dont add and remove the clue for a player, just switch to deduction mode, remembering the clue.
                {
                    let clue = self
                        .clues
                        .entry(player)
                        .or_insert(Clue::Terrain(Terrain::Desert));
                    let known = self.known_clues.entry(player).or_default();
                    ui.horizontal(|ui| {
                        ui.label(self.players.get(player).name.to_string());
                        ui.toggle_value(known, "Known");
                    });
                    if *known {
                        // Change clue type
                        egui::ComboBox::new(format!("combobox-clue-{player:?}"), "")
                            .selected_text("Edit type")
                            .show_ui(ui, |ui| {
                                if ui.button("Within one space of terrain").clicked() {
                                    *clue = Clue::Terrain(Terrain::Desert);
                                }
                                if ui.button("One of two terrains").clicked() {
                                    *clue = Clue::TwoTerrains(Terrain::Desert, Terrain::Forest);
                                }
                                if ui.button("Within one space of either animal").clicked() {
                                    *clue = Clue::EitherAnimal;
                                }
                                if ui.button("Within two spaces of animal").clicked() {
                                    *clue = Clue::Animal(Animal::Bear);
                                }
                                if ui.button("Within two spaces of structure type").clicked() {
                                    *clue = Clue::StructureKind(StructureKind::Shack);
                                }
                                if ui
                                    .button("Within three spaces of structure color")
                                    .clicked()
                                {
                                    *clue = Clue::StructureColor(StructureColor::Black);
                                }
                            });

                        // Edit clue
                        match clue {
                            Clue::Terrain(terrain) => {
                                ui.horizontal(|ui| {
                                    ui.label("Within one space of");
                                    terrain_switcher(format!("terrain-{player:?}"), ui, terrain);
                                });
                            }
                            Clue::TwoTerrains(a, b) => {
                                ui.horizontal(|ui| {
                                    ui.label("On");
                                    terrain_switcher(format!("terrain-{player:?}-a"), ui, a);
                                    ui.label("or");
                                    terrain_switcher(format!("terrain-{player:?}-b"), ui, b);
                                });
                            }
                            Clue::EitherAnimal => {
                                ui.label("Within one space of either animal");
                            }
                            Clue::Animal(animal) => {
                                ui.horizontal(|ui| {
                                    ui.label("Within two spaces of");
                                    egui::ComboBox::new(format!("animal-{player:?}"), "Territory")
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
                                    egui::ComboBox::new(format!("structurekind-{player:?}"), "")
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
                                    egui::ComboBox::new(
                                        format!("structurecolor-{player:?}"),
                                        "structure",
                                    )
                                    .selected_text(format!("{color}"))
                                    .show_ui(ui, |ui| {
                                        for c in StructureColor::iter() {
                                            ui.selectable_value(color, c, format!("{c}"));
                                        }
                                    });
                                });
                            }
                        }
                    } else {
                        // Show deduced clues.
                        // TODO Cache clues and only recompute when the answers change.
                        let clues = self.map.clues_for_player(player);
                        egui::CollapsingHeader::new(format!("{} possible clues", clues.len()))
                            .id_source(player)
                            .show(ui, |ui| {
                                for clue in clues {
                                    ui.label(format!("{clue}"));
                                }
                            });
                    }
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

        // Mark any tiles as small that violate known clues.
        for known_clue in self.players.iter().filter_map(|p| {
            if self.known_clues.get(&p.id).copied().unwrap_or_default() {
                self.clues.get(&p.id).copied()
            } else {
                None
            }
        }) {
            for i in 0..self.map.0.len() {
                let position = self.map.0[i].position;
                let found = self.map.clue_applies(known_clue, position);
                if !found {
                    self.map.0[i].small = true;
                }
            }
        }

        // Mark any tiles as small that violate deduced clues.
        // This is only the case if no clues for a player apply to the given tile.
        for i in 0..self.map.0.len() {
            let position = self.map.0[i].position;
            for player in self.players.iter() {
                let mut found_any = false;
                for clue in self.map.clues_for_player(player.id) {
                    if self.map.clue_applies(clue, position) {
                        found_any = true;
                        break;
                    }
                }
                if !found_any {
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
