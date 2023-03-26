use std::{collections::HashMap, hash::Hash};

use hexx::Hex;
use itertools::Itertools;
use notan::egui::{self, Grid, Label};
use strum::IntoEnumIterator;

use crate::{
    model::{
        Animal, Answer, Clue, ClueKind, Hint, Map, PlayerID, PlayerList, StructureColor,
        StructureKind, Terrain, Tile,
    },
    LAYOUT_SPACE,
};

use super::{placingstructures::PlacingStructures, Common};

#[derive(Debug)]
pub struct TryingClues {
    map: Map,
    /// Manually entered clues
    clues: HashMap<PlayerID, Clue>,
    /// Map from player to a bool. True: We know the clue; False: The clue should be deduced.
    known_clues: HashMap<PlayerID, bool>,
    /// Cache for clues deduced from answers.
    deduced_clues: HashMap<PlayerID, Vec<Clue>>,
    /// True if the game is played with inverted clues.
    with_inverted: bool,
    highlights: Vec<Hex>,
    players: PlayerList,
    hints: Vec<Hint>,
    /// The player that is using this software. Used for cheating from the correct perspective.
    user: PlayerID,
}

impl From<&PlacingStructures> for TryingClues {
    fn from(value: &PlacingStructures) -> Self {
        let players = value.players.clone();
        let user = players
            .iter()
            .next()
            .map(|p| p.id)
            .expect("empty PlayerList is not possible");

        let mut s = Self {
            map: Map(value.tiles().to_vec()),
            highlights: Vec::new(),
            players,
            clues: Default::default(),
            known_clues: Default::default(),
            deduced_clues: Default::default(),
            hints: Default::default(),
            user,
            with_inverted: false,
        };

        s.deduce_clues();
        s.update_map_from_clues();
        // We are using the entry API and setting default answers every time a tile is clicked.
        // Since that triggers recomputations of things, we just set all answers to unknown here for every tile.
        // That way no changes to the map are made when tiles are clicked.
        s.prefill_answers();
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

    fn gui(&mut self, ui: &mut egui::Ui) -> bool {
        let clues_before = self.clues.clone();
        let known_clues_before = self.known_clues.clone();
        let tiles_before = self.tiles().to_vec();
        let user_before = self.user;
        let with_inverted_before = self.with_inverted;

        ui.checkbox(&mut self.with_inverted, "Enable inverted clues");

        self.gui_for_answers(ui);
        ui.add_space(LAYOUT_SPACE);
        self.gui_for_cheats(ui);
        ui.add_space(LAYOUT_SPACE);
        self.gui_for_clues(ui);

        let clues_changed = clues_before != self.clues;
        let known_clues_changed = known_clues_before != self.known_clues;
        let tiles_changed = !itertools::equal(&tiles_before, self.tiles());
        let user_changed = user_before != self.user;
        let with_inverted_changed = with_inverted_before != self.with_inverted;

        if tiles_changed || with_inverted_changed {
            // The tiles i.e. the answers have changed so we need to think about the possible clues again.
            self.deduce_clues();
        }

        if clues_changed || known_clues_changed || tiles_changed || with_inverted_changed {
            self.update_map_from_clues();
        }

        if clues_changed
            || known_clues_changed
            || tiles_changed
            || user_changed
            || with_inverted_changed
        {
            // Something changed that influences the hints. Recomputing those is expensive,
            // so just clear them. The user can refresh them by pressing a button.
            self.hints.clear();
        }

        false
    }

    fn highlights(&self) -> Vec<Hex> {
        self.highlights.to_vec()
    }

    fn click(&mut self, hex: Hex) {
        self.highlights = self
            .map
            .get(hex)
            .is_some()
            .then_some(hex)
            .into_iter()
            .collect();
    }

    fn players(&self) -> &PlayerList {
        &self.players
    }
}

impl TryingClues {
    fn gui_for_cheats(&mut self, ui: &mut egui::Ui) {
        ui.heading("Cheat");
        ui.horizontal(|ui| {
            ui.label("You are");
            egui::ComboBox::new("cheat-player-select", "")
                .selected_text(&self.players.get(self.user).name)
                .show_ui(ui, |ui| {
                    for player in self.players.iter() {
                        ui.selectable_value(&mut self.user, player.id, &player.name);
                    }
                });
        });

        if self.hints.is_empty() {
            ui.horizontal(|ui| {
                if ui.button("Refresh").clicked() {
                    self.calculate_hints();
                }
                ui.add(Label::new("No hints available or map changed.").wrap(true));
            });
        }

        for hint in &self.hints {
            ui.horizontal(|ui| {
                if ui.button("Show").clicked() {
                    self.highlights = hint.tiles.to_vec();
                }
                ui.add(Label::new(&hint.text).wrap(true));
            });
        }
    }

    fn gui_for_answers(&mut self, ui: &mut egui::Ui) {
        // Answers can only be placed when there is a single selection.
        let selection = if self.highlights.len() == 1 {
            self.highlights.first().copied()
        } else {
            None
        };

        ui.heading("Answers");
        if let Some(selected_tile) = selection.and_then(|hex| self.map.get_mut(hex)) {
            ui.label("Set answers for the selected tile.");
            Grid::new("answer-grid").show(ui, |ui| {
                for player in self.players.iter() {
                    let answer = selected_tile.answers.entry(player.id).or_default();
                    ui.label(&player.name);
                    egui::ComboBox::new(format!("player-answer-{:?}", player.id), "")
                        .selected_text(format!("{answer}"))
                        .show_ui(ui, |ui| {
                            for a in Answer::iter() {
                                ui.selectable_value(answer, a, format!("{a}"));
                            }
                        });
                    ui.end_row();
                }
            });
        } else {
            ui.label("Select a tile to place anwers.");
        }
    }

    fn gui_for_clues(&mut self, ui: &mut egui::Ui) {
        let remaining_tiles = self.map.0.iter().filter(|t| !t.small).count();

        ui.heading("Clues");
        ui.label(format!("{remaining_tiles} tiles remain."));

        for player in self.players.iter().map(|p| p.id) {
            ui.separator();
            // Dont add and remove the clue for a player, just switch to deduction mode, remembering the clue.
            {
                let clue = self
                    .clues
                    .entry(player)
                    .or_insert(ClueKind::Terrain(Terrain::Desert).into());
                let known = self.known_clues.entry(player).or_default();
                ui.horizontal(|ui| {
                    ui.label(self.players.get(player).name.to_string());
                    ui.checkbox(known, "Known Clue");
                });
                if *known {
                    // Change clue type
                    egui::ComboBox::new(format!("combobox-clue-{player:?}"), "")
                        .selected_text("Edit type")
                        .show_ui(ui, |ui| {
                            if ui.button("Within one space of terrain").clicked() {
                                *clue = ClueKind::Terrain(Terrain::Desert).into();
                            }
                            if ui.button("One of two terrains").clicked() {
                                *clue =
                                    ClueKind::TwoTerrains(Terrain::Desert, Terrain::Forest).into();
                            }
                            if ui.button("Within one space of either animal").clicked() {
                                *clue = ClueKind::EitherAnimal.into();
                            }
                            if ui.button("Within two spaces of animal").clicked() {
                                *clue = ClueKind::Animal(Animal::Bear).into();
                            }
                            if ui.button("Within two spaces of structure type").clicked() {
                                *clue = ClueKind::StructureKind(StructureKind::Shack).into();
                            }
                            if ui
                                .button("Within three spaces of structure color")
                                .clicked()
                            {
                                *clue = ClueKind::StructureColor(StructureColor::Black).into();
                            }
                        });

                    // Edit clue
                    match &mut clue.kind {
                        ClueKind::Terrain(terrain) => {
                            ui.horizontal(|ui| {
                                ui.label("Within one space of");
                                terrain_switcher(format!("terrain-{player:?}"), ui, terrain);
                            });
                        }
                        ClueKind::TwoTerrains(a, b) => {
                            ui.horizontal(|ui| {
                                ui.label("On");
                                terrain_switcher(format!("terrain-{player:?}-a"), ui, a);
                                ui.label("or");
                                terrain_switcher(format!("terrain-{player:?}-b"), ui, b);
                            });
                        }
                        ClueKind::EitherAnimal => {
                            ui.label("Within one space of either animal");
                        }
                        ClueKind::Animal(animal) => {
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
                        ClueKind::StructureKind(kind) => {
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
                        ClueKind::StructureColor(color) => {
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
                    let clues = self.deduced_clues.entry(player).or_default();
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
    }

    fn prefill_answers(&mut self) {
        for tile in self.map.0.iter_mut() {
            for player in self.players.iter() {
                tile.answers.insert(player.id, Answer::Unknown);
            }
        }
    }

    /// Build a list of possible clues for each player according to their given answers.
    fn deduce_clues(&mut self) {
        for player in self.players.iter() {
            let clues = self.map.clues_for_player(player.id, self.with_inverted);
            self.deduced_clues.insert(player.id, clues);
        }
    }

    /// Calculate hints. This is compute intensive, so don't call it every frame.
    fn calculate_hints(&mut self) {
        self.hints.clear();

        /// Helper struct to keep track of how many clues/tiles are affected by asking
        /// a question on a tile.
        struct Question {
            tile: Hex,
            gain_with_no: usize,
            gain_with_yes: usize,
        }

        let opponents = self.players.iter().filter(|p| p.id != self.user);
        for player in opponents {
            let mut questions: Vec<Question> = Vec::new();

            // Simulate placing answers to find spaces with best chance of reducing clues.
            let clues_before = self.map.clues_for_player(player.id, self.with_inverted);
            if clues_before.len() == 1 {
                // Player has only a single clue left. No point in asking any questions.
                continue;
            }

            // Scan all tiles for quality of asking a question there.
            for i in 0..self.map.0.len() {
                let answer_before = *self.map.0[i].answers.entry(player.id).or_default();
                if answer_before != Answer::Unknown {
                    // Player already answered on this tile.
                    continue;
                }

                self.map.0[i].answers.insert(player.id, Answer::Yes);
                let clues_with_yes = self.map.clues_for_player(player.id, self.with_inverted);
                self.map.0[i].answers.insert(player.id, Answer::No);
                let clues_with_no = self.map.clues_for_player(player.id, self.with_inverted);
                self.map.0[i].answers.insert(player.id, Answer::Unknown);

                let gain_with_yes = clues_before.len().abs_diff(clues_with_yes.len());
                let gain_with_no = clues_before.len().abs_diff(clues_with_no.len());

                questions.push(Question {
                    tile: self.map.0[i].position,
                    gain_with_yes,
                    gain_with_no,
                });
            }

            // Perform binary search on available clues. Prefer questions that halve the available clues,
            // regardless of whether they answer yes or no.
            let best = questions
                .into_iter()
                .min_set_by_key(|q| q.gain_with_yes.abs_diff(q.gain_with_no));
            if let Some(q) = best.first() {
                let at_least = q.gain_with_no.min(q.gain_with_yes);
                let at_most = q.gain_with_no.max(q.gain_with_yes);
                let text = if at_least == at_most {
                    format!("Ask {} here to rule out {at_least} clues.", player.name)
                } else {
                    format!(
                        "Ask {} here to rule out {at_least} to {at_most} clues.",
                        player.name
                    )
                };
                let tiles = best.into_iter().map(|q| q.tile).collect();
                self.hints.push(Hint { text, tiles });
            }
        }

        // Find tiles that give the least information (change in possible clues
        // when the user is forced to place a "no".
        // TODO Recursive checks? Say there are two fields A and B that reveal no clues when a
        // "no" is placed on them. But after that another "no" might need to be placed, and maybe
        // A would allow me to reveal no new information again, while choosing B forces me to rule out
        // new clues now.
        struct No {
            clue_diff: usize,
            tile: Hex,
        }
        let mut nos = Vec::new();
        let clues_before = self.map.clues_for_player(self.user, self.with_inverted);
        for i in 0..self.map.0.len() {
            let answer_before = *self.map.0[i].answers.entry(self.user).or_default();
            if answer_before != Answer::Unknown {
                // Player already answered on this tile.
                continue;
            }

            self.map.0[i].answers.insert(self.user, Answer::No);
            let clues_with_no = self.map.clues_for_player(self.user, self.with_inverted);
            self.map.0[i].answers.insert(self.user, Answer::Unknown);

            nos.push(No {
                clue_diff: clues_before.len().abs_diff(clues_with_no.len()),
                tile: self.map.0[i].position,
            });
        }
        let best = nos.into_iter().min_set_by_key(|n| n.clue_diff);
        if let Some(diff) = best.first().map(|n| n.clue_diff) {
            let text = if diff == 0 {
                "Place a 'no' here to reveal no new information.".to_owned()
            } else {
                format!("Place a 'no' here to rule out {diff} of your clues.")
            };
            let tiles = best.into_iter().map(|n| n.tile).collect();
            self.hints.push(Hint { text, tiles });
        }
    }

    /// Go through all tiles and see if any clue applies to them.
    /// If no clue applies to them, they are drawn as small.
    fn update_map_from_clues(&mut self) {
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
                for clue in self.deduced_clues.entry(player.id).or_default() {
                    if self.map.clue_applies(*clue, position) {
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
