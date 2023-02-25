use std::{collections::VecDeque, hash::Hash, iter};

use notan::egui::{self, Context};
use strum::IntoEnumIterator;

use crate::{
    buildingmap,
    model::{Clue, Map, Terrain, Tile, Animal},
};

#[derive(Debug)]
pub struct SubState {
    map: Map,
    clues: Vec<Clue>,
}

impl From<&buildingmap::SubState> for SubState {
    fn from(value: &buildingmap::SubState) -> Self {
        let mut s = Self {
            map: Map(value.tiles().to_vec()),
            clues: Vec::new(),
        };
        s.scan_clues();
        s
    }
}

impl SubState {
    /// Go through all tiles and see if any clue applies to them.
    /// If no clue applies to them, they are drawn as small.
    pub fn scan_clues(&mut self) {
        // Set tile to be big. Should any clue fail, then it will be small.
        for tile in &mut self.map.0 {
            tile.small = false;
        }

        for &clue in &self.clues {
            for i in 0..self.map.0.len() {
                match clue {
                    Clue::WithinOneTerrain(terrain) => {
                        let pos = self.map.0[i].position;
                        let found = self.map.any(pos, 1, |t| t.terrain == terrain);
                        if !found {
                            self.map.0[i].small = true;
                        }
                    }
                    Clue::TwoTerrains(a, b) => {
                        let tile = &mut self.map.0[i];
                        if tile.terrain != a && tile.terrain != b {
                            tile.small = true;
                        }
                    }
                    Clue::OneSpaceAnimal => {
                        let pos = self.map.0[i].position;
                        let found = self.map.any(pos, 1, |t| t.animal.is_some());
                        if !found {
                            self.map.0[i].small = true;
                        }
                    }
                    Clue::TwoSpaceAnimal(animal) => {
                        let pos = self.map.0[i].position;
                        let found = self.map.any(pos, 2, |t| t.animal == Some(animal));
                        if !found {
                            self.map.0[i].small = true;
                        }
                    },
                }
            }
        }
    }

    pub fn tiles(&self) -> &[Tile] {
        &self.map.0
    }

    pub fn gui(&mut self, ctx: &Context) -> bool {
        let clues_before = self.clues.clone();

        // Index of clue to delete
        let mut delete_index = None;

        egui::Window::new("Clues").show(ctx, |ui| {
            // Edit existing clues
            for (i, clue) in self.clues.iter_mut().enumerate() {
                if i > 0 {
                    ui.separator();
                }

                match clue {
                    Clue::WithinOneTerrain(terrain) => {
                        ui.label("Within one space of");
                        terrain_switcher(format!("terrain-{i}"), ui, terrain);
                    }
                    Clue::TwoTerrains(a, b) => {
                        ui.label("Either on");
                        terrain_switcher(format!("terrain-{i}-a"), ui, a);
                        ui.label("or on");
                        terrain_switcher(format!("terrain-{i}-b"), ui, b);
                    }
                    Clue::OneSpaceAnimal => {
                        ui.label("Within one space of either animal");
                    }
                    Clue::TwoSpaceAnimal(animal) => {
                        ui.label("Within two spaces of");
                        egui::ComboBox::new(format!("animal-{i}"), "Territory")
                            .selected_text(format!("{animal}"))
                            .show_ui(ui, |ui| {
                                for a in Animal::iter() {
                                    ui.selectable_value(animal, a, format!("{a}"));
                                }
                            });
                    },
                    
                }

                if ui.button("Delete").clicked() {
                    delete_index = Some(i);
                }
            }

            if !self.clues.is_empty() {
                ui.separator();
            }

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

// Dropdown for switching terrain types.
fn terrain_switcher(id: impl Hash, ui: &mut egui::Ui, terrain: &mut Terrain) {
    egui::ComboBox::new(id, "Terrain")
        .selected_text(format!("{terrain}"))
        .show_ui(ui, |ui| {
            for t in Terrain::iter() {
                ui.selectable_value(terrain, t, format!("{t}"));
            }
        });
}
