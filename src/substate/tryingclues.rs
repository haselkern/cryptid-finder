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

        // TODO This UI needs to be revamped to better incorporate players and their given answers.

        egui::Window::new("Clues").show(ctx, |ui| {
            ui.label(format!("{remaining_tiles} tiles remain."));

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
        });

        if let Some(delete) = delete_index {
            self.clues.remove(delete);
        }
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
                let pos = self.map.0[i].position;
                let found = match clue {
                    Clue::Terrain(terrain) => self.map.any(pos, 1, |t| t.terrain == terrain),
                    Clue::TwoTerrains(a, b) => {
                        let tile = &self.map.0[i];
                        tile.terrain == a || tile.terrain == b
                    }
                    Clue::EitherAnimal => self.map.any(pos, 1, |t| t.animal.is_some()),
                    Clue::Animal(animal) => {
                        let pos = self.map.0[i].position;
                        self.map.any(pos, 2, |t| t.animal == Some(animal))
                    }
                    Clue::StructureKind(kind) => self.map.any(pos, 2, |t| {
                        t.structure.map(|s| s.kind == kind).unwrap_or(false)
                    }),
                    Clue::StructureColor(color) => self.map.any(pos, 3, |t| {
                        t.structure.map(|s| s.color == color).unwrap_or(false)
                    }),
                };
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
