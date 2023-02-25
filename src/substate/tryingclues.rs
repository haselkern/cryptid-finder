use std::hash::Hash;

use notan::egui::{self, Context};
use strum::IntoEnumIterator;

use crate::{
    model::{Animal, Clue, Map, StructureColor, StructureKind, Terrain, Tile},
    substate::placingstructures,
};

use super::Common;

#[derive(Debug)]
pub struct TryingClues {
    map: Map,
    clues: Vec<Clue>,
}

impl From<&placingstructures::PlacingStructures> for TryingClues {
    fn from(value: &placingstructures::PlacingStructures) -> Self {
        let mut s = Self {
            map: Map(value.tiles().to_vec()),
            clues: Vec::new(),
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

        // Index of clue to delete
        let mut delete_index = None;

        let remaining_tiles = self.map.0.iter().filter(|t| !t.small).count();

        egui::Window::new("Clues").show(ctx, |ui| {
            ui.label(format!("{remaining_tiles} tiles remain."));

            // Edit existing clues
            for (i, clue) in self.clues.iter_mut().enumerate() {
                ui.separator();

                match clue {
                    Clue::WithinOneTerrain(terrain) => {
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
                    Clue::OneSpaceAnimal => {
                        ui.label("Within one space of either animal");
                    }
                    Clue::TwoSpaceAnimal(animal) => {
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
                    Clue::TwoSpaceStructureKind(kind) => {
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
                    Clue::ThreeSpaceStructureColor(color) => {
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
                        self.clues.push(Clue::WithinOneTerrain(Terrain::Desert));
                    }
                    if ui.button("One of two terrains").clicked() {
                        self.clues
                            .push(Clue::TwoTerrains(Terrain::Desert, Terrain::Forest));
                    }
                    if ui.button("Within one space of either animal").clicked() {
                        self.clues.push(Clue::OneSpaceAnimal);
                    }
                    if ui.button("Within two spaces of animal").clicked() {
                        self.clues.push(Clue::TwoSpaceAnimal(Animal::Bear));
                    }
                    if ui.button("Within two spaces of structure type").clicked() {
                        self.clues
                            .push(Clue::TwoSpaceStructureKind(StructureKind::Shack));
                    }
                    if ui
                        .button("Within three spaces of structure color")
                        .clicked()
                    {
                        self.clues
                            .push(Clue::ThreeSpaceStructureColor(StructureColor::Black));
                    }
                });
        });

        if let Some(delete) = delete_index {
            self.clues.remove(delete);
        }

        if clues_before != self.clues {
            self.scan_clues();
        }

        false
    }
}

impl TryingClues {
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
                    Clue::WithinOneTerrain(terrain) => {
                        self.map.any(pos, 1, |t| t.terrain == terrain)
                    }
                    Clue::TwoTerrains(a, b) => {
                        let tile = &mut self.map.0[i];
                        tile.terrain != a && tile.terrain != b
                    }
                    Clue::OneSpaceAnimal => self.map.any(pos, 1, |t| t.animal.is_some()),
                    Clue::TwoSpaceAnimal(animal) => {
                        let pos = self.map.0[i].position;
                        self.map.any(pos, 2, |t| t.animal == Some(animal))
                    }
                    Clue::TwoSpaceStructureKind(kind) => self.map.any(pos, 2, |t| {
                        t.structure.map(|s| s.kind == kind).unwrap_or(false)
                    }),
                    Clue::ThreeSpaceStructureColor(color) => self.map.any(pos, 3, |t| {
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