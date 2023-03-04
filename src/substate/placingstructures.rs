use hexx::Hex;
use notan::egui::{self, Align, Layout};
use strum::IntoEnumIterator;

use crate::{
    model::{PlayerList, Structure, StructureColor, StructureKind, Tile},
    LAYOUT_SPACE,
};

use super::{buildingmap::BuildingMap, Common};

#[derive(Debug)]
pub struct PlacingStructures {
    map: Vec<Tile>,
    pub players: PlayerList,
}

impl From<&BuildingMap> for PlacingStructures {
    fn from(value: &BuildingMap) -> Self {
        let mut s = Self {
            map: value.tiles().to_vec(),
            players: value.players.clone(),
        };

        // Add default colors
        s.add(StructureColor::White);
        s.add(StructureColor::Green);
        s.add(StructureColor::Blue);

        s
    }
}

impl Common for PlacingStructures {
    fn tiles(&self) -> &[Tile] {
        &self.map
    }
    fn tiles_mut(&mut self) -> &mut [Tile] {
        &mut self.map
    }

    fn gui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut next_state = false;

        ui.heading("Structures");
        for color in StructureColor::iter() {
            let mut has = self.has(color);
            if ui
                .checkbox(&mut has, format!("{color} structures"))
                .changed()
            {
                if has {
                    self.add(color);
                } else {
                    self.delete(color);
                }
            }
        }

        ui.add_space(LAYOUT_SPACE);
        ui.label("Drag structures into position on the map.");
        ui.add_space(LAYOUT_SPACE);

        ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
            if ui.button("Ready").clicked() {
                next_state = true;
            }
        });

        next_state
    }

    fn highlights(&self) -> Vec<Hex> {
        Vec::new()
    }

    fn click(&mut self, _hex: Hex) {}

    fn players(&self) -> &PlayerList {
        &self.players
    }
}

impl PlacingStructures {
    /// Returns true if the structure color is present.
    fn has(&self, color: StructureColor) -> bool {
        self.map
            .iter()
            .filter_map(|t| t.structure)
            .any(|s| s.color == color)
    }

    /// Add the structures for the given color to the map.
    fn add(&mut self, color: StructureColor) {
        let mut to_add = vec![
            Structure {
                kind: StructureKind::Shack,
                color,
            },
            Structure {
                kind: StructureKind::Stone,
                color,
            },
        ];

        // Find free spaces to add those structures
        let mut i = 0;
        while let Some(to_add) = to_add.pop() {
            while self.map[i].structure.is_some() {
                i += 1;
            }
            self.map[i].structure = Some(to_add);
        }
    }

    /// Delete the structures for the given color from the map.
    fn delete(&mut self, color: StructureColor) {
        for tile in &mut self.map {
            let Some(structure) = tile.structure else {
                continue;
            };

            if structure.color == color {
                tile.structure = None;
            }
        }
    }
}
