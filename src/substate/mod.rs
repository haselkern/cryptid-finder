use enum_dispatch::enum_dispatch;
use notan::egui;

use crate::model::Tile;

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
    fn gui(&mut self, ctx: &egui::Context) -> bool;
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