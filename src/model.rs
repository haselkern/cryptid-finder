use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use hexx::{Hex, HexMap, OffsetHexMode};
use itertools::Itertools;
use notan::{egui, prelude::Color};
use strum::{Display, EnumIter, IntoEnumIterator};

#[derive(Debug, Clone, Copy, EnumIter, PartialEq, Eq, Hash, Display)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Display)]
pub enum Animal {
    Bear,
    Cougar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Display, Hash)]
pub enum StructureColor {
    White,
    Green,
    Blue,
    Black,
}

impl From<StructureColor> for Color {
    fn from(value: StructureColor) -> Self {
        match value {
            StructureColor::White => Color::new(0.9, 0.9, 0.9, 1.0),
            StructureColor::Green => Color::new(0.2, 0.8, 0.2, 1.0),
            StructureColor::Blue => Color::new(0.2, 0.2, 0.8, 1.0),
            StructureColor::Black => Color::new(0.1, 0.1, 0.1, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Display, Hash)]
pub enum StructureKind {
    #[strum(to_string = "Abandoned Shack")]
    Shack,
    #[strum(to_string = "Standing Stone")]
    Stone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Structure {
    pub kind: StructureKind,
    pub color: StructureColor,
}

/// A single hexagon in the game world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile {
    pub position: Hex,
    pub terrain: Terrain,
    pub animal: Option<Animal>,
    pub structure: Option<Structure>,
    /// Small is true if this tile should be drawn a bit smaller than usual.
    pub small: bool,
    /// Answers given by players questioning this tile.
    /// TODO Use BTreeMap to have consistent order when rendering.
    pub answers: HashMap<PlayerID, Answer>,
}

/// Choice for building the world. User can select a piece and decide to rotate it 180°.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PieceChoice {
    pub piece: Piece,
    pub rotated: bool,
}

impl fmt::Display for PieceChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
                    structure: None, // Structures get added later
                    small: false,
                    answers: HashMap::new(),
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

/// All possible clues.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Clue {
    /// The creature is with one space of the terrain.
    Terrain(Terrain),
    /// The creature is on one of these types of terrain.
    TwoTerrains(Terrain, Terrain),
    /// The creature is within one space of either animal.
    EitherAnimal,
    /// The creature is within two spaces of the animal territory.
    Animal(Animal),
    /// The creature is within two spaces of the type of structure.
    StructureKind(StructureKind),
    /// The creature is within two spaces of the type of structure.
    StructureColor(StructureColor),
}

impl Clue {
    /// Returns every possible clue for the available structure colors/kinds.
    pub fn all(
        structure_colors: impl IntoIterator<Item = StructureColor>,
        structure_kinds: impl IntoIterator<Item = StructureKind>,
    ) -> impl Iterator<Item = Self> {
        let terrain = Terrain::iter().map(Clue::Terrain);
        let two_terrains = Terrain::iter()
            .combinations(2)
            .map(|ts| Clue::TwoTerrains(ts[0], ts[1]));
        let either_animal = [Clue::EitherAnimal];
        let animal = Animal::iter().map(Clue::Animal);
        let structure_kind = structure_kinds.into_iter().map(Clue::StructureKind);
        let structure_color = structure_colors.into_iter().map(Clue::StructureColor);

        terrain
            .chain(two_terrains)
            .chain(either_animal)
            .chain(animal)
            .chain(structure_kind)
            .chain(structure_color)
    }
}

impl fmt::Display for Clue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Clue::Terrain(t) => write!(f, "Within one space of {t}"),
            Clue::TwoTerrains(a, b) => write!(f, "On {a} or {b}"),
            Clue::EitherAnimal => write!(f, "Within one space of bear or cougar"),
            Clue::Animal(a) => write!(f, "Within two spaces of {a}"),
            Clue::StructureKind(k) => write!(f, "Within two spaces of {k}"),
            Clue::StructureColor(c) => write!(f, "Within three spaces of {c} structure"),
        }
    }
}

/// A map of tiles.
#[derive(Debug, Default)]
pub struct Map(pub Vec<Tile>);

impl Map {
    pub fn get(&self, at: Hex) -> Option<&Tile> {
        self.0.iter().find(|tile| tile.position == at)
    }

    pub fn get_mut(&mut self, at: Hex) -> Option<&mut Tile> {
        self.0.iter_mut().find(|tile| tile.position == at)
    }

    /// Returns true if the cryptid could be at the given position according to the clue.
    pub fn clue_applies(&self, clue: Clue, position: Hex) -> bool {
        match clue {
            Clue::Terrain(terrain) => self.any(position, 1, |t| t.terrain == terrain),
            Clue::TwoTerrains(a, b) => match self.get(position) {
                Some(tile) => tile.terrain == a || tile.terrain == b,
                None => false,
            },
            Clue::EitherAnimal => self.any(position, 1, |t| t.animal.is_some()),
            Clue::Animal(animal) => self.any(position, 2, |t| t.animal == Some(animal)),
            Clue::StructureKind(kind) => self.any(position, 2, |t| {
                t.structure.map(|s| s.kind == kind).unwrap_or(false)
            }),
            Clue::StructureColor(color) => self.any(position, 3, |t| {
                t.structure.map(|s| s.color == color).unwrap_or(false)
            }),
        }
    }

    /// Returns [StructureColor]s present on the map.
    pub fn structure_colors(&self) -> impl Iterator<Item = StructureColor> + '_ {
        self.0
            .iter()
            .filter_map(|t| t.structure)
            .map(|s| s.color)
            .unique()
    }

    /// Returns [StructureKind]s present on the map.
    pub fn structure_kinds(&self) -> impl Iterator<Item = StructureKind> + '_ {
        self.0
            .iter()
            .filter_map(|t| t.structure)
            .map(|s| s.kind)
            .unique()
    }

    /// Return a list of possible clues for the player, respecting the answers they already gave.
    pub fn clues_for_player(&self, player: PlayerID) -> Vec<Clue> {
        let mut result = Vec::new();

        for clue in Clue::all(self.structure_colors(), self.structure_kinds()) {
            let tiles_with_answer = self
                .0
                .iter()
                .filter_map(|t| t.answers.get(&player).map(|&a| (a, t)));

            // We now need to decide whether the clue is possible for the player by answering:
            //     Does the clue contradict any answers the player gave?

            let mut contradiction = false;
            for (answer, tile) in tiles_with_answer {
                let clue_applies = self.clue_applies(clue, tile.position);
                let contradicts = match (answer, clue_applies) {
                    (Answer::Unknown, _) => false,
                    (Answer::Yes, true) => false,
                    (Answer::Yes, false) => true,
                    (Answer::No, true) => true,
                    (Answer::No, false) => false,
                };
                if contradicts {
                    contradiction = true;
                    break;
                }
            }

            if !contradiction {
                result.push(clue);
            }
        }

        result
    }

    /// Check any fields for the condition. Position is always checked. Add fields with "distance".
    /// Distance 0 is only position. Distance 1 is position with direct neighbors, etc.
    /// Returns true if the condition is true for any field.
    pub fn any<F>(&self, position: Hex, distance: u32, condition: F) -> bool
    where
        F: Fn(&Tile) -> bool,
    {
        let to_check = HexMap::new(distance).with_center(position);
        for check in to_check.all_coords() {
            if let Some(tile) = self.get(check) {
                if condition(tile) {
                    return true;
                }
            }
        }
        false
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct PlayerID(usize);

#[derive(Debug, Clone)]
pub struct Player {
    pub id: PlayerID,
    pub name: String,
    pub color: PlayerColor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Hash, Display)]
pub enum PlayerColor {
    Red,
    Purple,
    Orange,
    Green,
    Blue,
}

impl From<PlayerColor> for egui::Color32 {
    fn from(value: PlayerColor) -> Self {
        match value {
            PlayerColor::Red => Self::from_rgb(204, 52, 36),
            PlayerColor::Purple => Self::from_rgb(135, 87, 156),
            PlayerColor::Orange => Self::from_rgb(246, 159, 38),
            PlayerColor::Green => Self::from_rgb(38, 158, 117),
            PlayerColor::Blue => Self::from_rgb(85, 197, 223),
        }
    }
}

impl From<PlayerColor> for Color {
    fn from(value: PlayerColor) -> Self {
        let color: egui::Color32 = value.into();
        Color::from(color.to_array())
    }
}

/// Answer a player gave on a tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, Display)]
pub enum Answer {
    /// The player gave no information for a tile.
    Unknown,
    /// The player revealed that the cryptid may be on the tile in question.
    Yes,
    /// The player revealed that the cryptid cannot be on the tile in question.
    No,
}

impl Default for Answer {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Default)]
pub struct PlayerList(Vec<Player>);

impl PlayerList {
    pub fn get(&self, id: PlayerID) -> &Player {
        self.0
            .iter()
            .find(|p| p.id == id)
            .unwrap_or_else(|| panic!("Invalid {id:?} provided"))
    }

    pub fn remove(&mut self, id: PlayerID) {
        self.0.retain(|p| p.id != id);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Player> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Player> {
        self.0.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn push_new(&mut self) {
        let id = self.0.iter().map(|p| p.id.0).max().unwrap_or(0) + 1;
        let all_colors: HashSet<PlayerColor> = PlayerColor::iter().collect();
        let taken_colors: HashSet<PlayerColor> = self.0.iter().map(|p| p.color).collect();
        let possible_colors = all_colors.difference(&taken_colors);
        let color = possible_colors
            .into_iter()
            .copied()
            .next()
            .unwrap_or(PlayerColor::Red);

        self.0.push(Player {
            id: PlayerID(id),
            name: "Some Player".to_owned(),
            color,
        })
    }
}
