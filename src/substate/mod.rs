use enum_dispatch::enum_dispatch;
use hexx::Hex;
use notan::egui;

use crate::model::{PlayerList, Tile};

mod buildingmap;
mod placingstructures;
mod tryingclues;

pub use buildingmap::BuildingMap;
pub use placingstructures::PlacingStructures;
pub use tryingclues::TryingClues;

#[enum_dispatch]
pub trait Common {
    /// Tiles to draw in the window.
    fn tiles(&self) -> &[Tile];
    fn tiles_mut(&mut self) -> &mut [Tile];
    /// Show an egui. Return true to switch to the next state.
    fn gui(&mut self, ui: &mut egui::Ui) -> bool;
    /// Draw a highlight around a tile, if needed.
    fn highlights(&self) -> Vec<Hex>;
    /// Click on a tile.
    fn click(&mut self, hex: Hex);
    fn players(&self) -> &PlayerList;
}

#[enum_dispatch(Common)]
#[derive(Debug)]
pub enum SubState {
    BuildingMap,
    PlacingStructures,
    TryingClues,
}

impl Default for SubState {
    fn default() -> Self {
        Self::BuildingMap(BuildingMap::default())
    }
}
