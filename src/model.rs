use std::fmt::Display;

use hexx::{Hex, OffsetHexMode};
use notan::prelude::Color;
use strum::EnumIter;

#[derive(Debug, Clone, Copy, EnumIter, PartialEq, Eq, Hash)]
pub enum Terrain {
    Desert,
    Forest,
    Water,
    Swamp,
    Mountain,
}

impl From<Terrain> for Color {
    fn from(value: Terrain) -> Self {
        match value {
            Terrain::Desert => Color::new(0.82, 0.7, 0.31, 1.0),
            Terrain::Forest => Color::new(0.2, 0.35, 0.24, 1.0),
            Terrain::Water => Color::new(0.31, 0.5, 0.78, 1.0),
            Terrain::Swamp => Color::new(0.23, 0.18, 0.29, 1.0),
            Terrain::Mountain => Color::new(0.62, 0.62, 0.62, 1.0),
        }
    }
}

#[derive(Debug)]
pub enum PlayerKind {
    Alpha,
    Beta,
    Gamma,
    Delta,
    Epsilon,
}

#[derive(Debug, Clone, Copy)]
pub enum Animal {
    Bear,
    Cougar,
}

#[derive(Debug, Clone, Copy)]
pub enum BuildingColor {
    White,
    Green,
    Blue,
    Black,
}

impl From<BuildingColor> for Color {
    fn from(value: BuildingColor) -> Self {
        match value {
            BuildingColor::White => Color::WHITE,
            BuildingColor::Green => Color::GREEN,
            BuildingColor::Blue => Color::BLUE,
            BuildingColor::Black => Color::BLACK,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BuildingKind {
    Shack,
    Stone,
}

#[derive(Debug, Clone, Copy)]
pub struct Building {
    pub kind: BuildingKind,
    pub color: BuildingColor,
}

/// A single hexagon in the game world.
#[derive(Debug)]
pub struct Tile {
    pub position: Hex,
    pub terrain: Terrain,
    pub animal: Option<Animal>,
    pub building: Option<Building>,
}

/// Choice for building the world. User can select a piece and decide to rotate it 180°.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PieceChoice {
    pub piece: Piece,
    pub rotated: bool,
}

impl Display for PieceChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.rotated {
            write!(f, "{} (rotated)", self.piece.name())
        } else {
            write!(f, "{}", self.piece.name())
        }
    }
}

impl From<Piece> for PieceChoice {
    fn from(piece: Piece) -> Self {
        Self {
            piece,
            rotated: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Hash)]
pub enum Piece {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
}

impl Piece {
    pub fn definition(self) -> &'static str {
        match self {
            Piece::One => include_str!("../assets/piece-1.txt"),
            Piece::Two => include_str!("../assets/piece-2.txt"),
            Piece::Three => include_str!("../assets/piece-3.txt"),
            Piece::Four => include_str!("../assets/piece-4.txt"),
            Piece::Five => include_str!("../assets/piece-5.txt"),
            Piece::Six => include_str!("../assets/piece-6.txt"),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Piece::One => "1",
            Piece::Two => "2",
            Piece::Three => "3",
            Piece::Four => "4",
            Piece::Five => "5",
            Piece::Six => "6",
        }
    }

    pub fn parse(self) -> ParsedPiece {
        let mut tiles = Vec::new();
        for (row_i, row) in self.definition().lines().enumerate() {
            let chars: Vec<char> = row.chars().collect();
            for col_i in (0..row.len()).step_by(2) {
                let terrain = chars[col_i];
                let animal = chars.get(col_i + 1).copied().unwrap_or(' '); // Be lenient with missing trailing spaces

                let terrain = match terrain {
                    'W' => Terrain::Water,
                    'D' => Terrain::Desert,
                    'M' => Terrain::Mountain,
                    'F' => Terrain::Forest,
                    'S' => Terrain::Swamp,
                    unknown => panic!("Terrain {unknown} invalid, must be one of WDMFS"),
                };

                let animal = match animal {
                    'b' => Some(Animal::Bear),
                    'c' => Some(Animal::Cougar),
                    _ => None,
                };

                tiles.push(Tile {
                    position: Hex::from_offset_coordinates(
                        [col_i as i32 / 2, row_i as i32],
                        OffsetHexMode::OddColumns,
                    ),
                    terrain,
                    animal,
                    building: None, // buildings get added later
                });
            }
        }
        ParsedPiece(tiles)
    }
}

/// One of the six 6x3 pieces the world is built out of.
pub struct ParsedPiece(pub Vec<Tile>);

impl ParsedPiece {
    /// Rotate this piece 180°. The origin (0,0) is expected to be top-left
    /// and will be top left after rotating.
    pub fn rotate(&mut self) {
        for tile in self.0.iter_mut() {
            tile.position = -tile.position + Hex::new(5, 0);
        }
    }

    pub fn translate(&mut self, t: Hex) {
        for tile in self.0.iter_mut() {
            tile.position += t;
        }
    }
}
