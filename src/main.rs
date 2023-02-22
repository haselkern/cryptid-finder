use core::panic;
use std::{
    collections::{HashMap, HashSet},
    f32::consts::PI,
    fmt::Display,
};

use hexx::{Hex, HexLayout, HexOrientation};
use notan::{
    draw::{CreateDraw, DrawConfig, DrawImages, DrawShapes, DrawTransform},
    egui::{self, EguiConfig, EguiPluginSugar},
    math::{Mat3, Vec2},
    prelude::*,
};
use strum::{EnumIter, IntoEnumIterator};

#[derive(Debug, Clone, Copy, EnumIter, PartialEq, Eq, Hash)]
enum Terrain {
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
enum PlayerKind {
    Alpha,
    Beta,
    Gamma,
    Delta,
    Epsilon,
}

#[derive(Debug, Clone, Copy)]
enum Animal {
    Bear,
    Cougar,
}

#[derive(Debug, Clone, Copy)]
enum BuildingColor {
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
enum BuildingKind {
    Shack,
    Stone,
}

#[derive(Debug, Clone, Copy)]
struct Building {
    kind: BuildingKind,
    color: BuildingColor,
}

/// A single hexagon in the game world.
#[derive(Debug)]
struct Tile {
    position: Hex,
    terrain: Terrain,
    animal: Option<Animal>,
    building: Option<Building>,
}

#[derive(Debug)]
struct World {
    tiles: Vec<Tile>,
}

/// Choice for building the world. User can select a piece and decide to rotate it 180°.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PieceChoice {
    piece: Piece,
    rotated: bool,
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

#[derive(AppState)]
struct State {
    // Radius of the tiles to draw
    tile_radius: f32,
    // Offset to draw the tiles at. Used for dragging with mouse.
    offset: Vec2,
    world: World,
    icons: HashMap<Terrain, Texture>,
    // TODO Split the state into substates. selected_pieces only exist during the setup for example.
    selected_pieces: [PieceChoice; 6],
    is_egui_hovered: bool,
    mouse_last_frame: Vec2,
}

impl State {
    fn new(gfx: &mut Graphics) -> Self {
        let icons = load_icons(gfx);
        let world = World {
            tiles: Piece::One.parse().0,
        };

        Self {
            tile_radius: 64.0,
            icons,
            world,
            selected_pieces: Piece::iter()
                .map(Into::into)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            is_egui_hovered: false,
            offset: Vec2::ZERO,
            mouse_last_frame: Vec2::ZERO,
        }
    }

    /// Returns true if every piece was selected exactly once.
    fn are_selected_pieces_valid(&self) -> bool {
        let pieces: HashSet<Piece> = self
            .selected_pieces
            .iter()
            .map(|choice| choice.piece)
            .collect();
        pieces.len() == 6
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Hash)]
enum Piece {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
}

impl Piece {
    fn definition(self) -> &'static str {
        match self {
            Piece::One => include_str!("../assets/piece-1.txt"),
            Piece::Two => include_str!("../assets/piece-2.txt"),
            Piece::Three => include_str!("../assets/piece-3.txt"),
            Piece::Four => include_str!("../assets/piece-4.txt"),
            Piece::Five => include_str!("../assets/piece-5.txt"),
            Piece::Six => include_str!("../assets/piece-6.txt"),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Piece::One => "1",
            Piece::Two => "2",
            Piece::Three => "3",
            Piece::Four => "4",
            Piece::Five => "5",
            Piece::Six => "6",
        }
    }

    fn parse(self) -> ParsedPiece {
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
                        hexx::OffsetHexMode::OddColumns,
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
struct ParsedPiece(Vec<Tile>);

impl ParsedPiece {
    // TODO Rotate
}

fn load_icons(gfx: &mut Graphics) -> HashMap<Terrain, Texture> {
    Terrain::iter()
        .map(|t| {
            (
                t,
                match t {
                    Terrain::Desert => include_bytes!("../assets/weather-sun.png").as_slice(),
                    Terrain::Forest => include_bytes!("../assets/wild-harvested.png").as_slice(),
                    Terrain::Water => include_bytes!("../assets/wave.png").as_slice(),
                    Terrain::Swamp => include_bytes!("../assets/skull.png").as_slice(),
                    Terrain::Mountain => include_bytes!("../assets/rocky-mountain.png").as_slice(),
                },
            )
        })
        .map(|(t, bytes)| {
            (
                t,
                gfx.create_texture()
                    .from_image(bytes)
                    .build()
                    .expect("load icon"),
            )
        })
        .collect()
}

#[notan_main]
fn main() -> Result<(), String> {
    notan::init_with(State::new)
        .draw(draw)
        .event(event)
        .add_config(DrawConfig)
        .add_config(EguiConfig)
        .add_config(WindowConfig::new().resizable(true).title("Cryptid Finder"))
        .build()
}

fn event(state: &mut State, event: Event) {
    if let Event::MouseWheel { delta_y, .. } = event {
        state.tile_radius = (state.tile_radius + delta_y).clamp(8.0, 1024.0);
    }
}

fn draw(app: &mut App, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
    let mut draw = gfx.create_draw();
    draw.clear(Color::BLACK);

    let stroke_width = state.tile_radius * 0.05;

    let (window_width, window_height) = app.window().size();
    let window_size: Vec2 = (window_width as f32, window_height as f32).into();
    let origin = window_size * 0.5 + state.offset;

    let layout = HexLayout {
        orientation: HexOrientation::flat(),
        origin,
        hex_size: Vec2::splat(state.tile_radius),
    };

    for tile in &state.world.tiles {
        let pos = layout.hex_to_world_pos(tile.position);
        draw.transform().push(Mat3::from_translation(pos));

        // Draw flat topped hex
        {
            draw.transform().push(Mat3::from_rotation_z(PI / 6.0));

            draw.polygon(6, state.tile_radius)
                .color(tile.terrain.into());

            if let Some(animal) = tile.animal {
                let color = match animal {
                    Animal::Bear => Color::BLACK,
                    Animal::Cougar => Color::RED,
                };

                draw.polygon(6, state.tile_radius * 0.9)
                    .stroke(stroke_width)
                    .stroke_color(color);
            }

            draw.transform().pop();
        }

        if let Some(building) = tile.building {
            let color = building.color.into();
            let sides = match building.kind {
                BuildingKind::Shack => 3,
                BuildingKind::Stone => 8,
            };

            draw.polygon(sides, state.tile_radius * 0.7)
                .color(color)
                .rotate(PI);
        }

        // Draw icon for terrain
        {
            let tex = state.icons.get(&tile.terrain).unwrap();
            let scale = state.tile_radius * 0.015;
            let size = Vec2::from(tex.size());
            draw.transform()
                .push(Mat3::from_scale(Vec2::splat(scale)) * Mat3::from_translation(size * -0.5));
            draw.image(tex).alpha(0.3);
            draw.transform().pop();
        }

        draw.transform().pop();
    }
    gfx.render(&draw);

    let output = plugins.egui(|ctx| {
        egui::Window::new("Map Setup").show(ctx, |ui| {
            egui::Grid::new("map-setup-grid").show(ui, |ui| {
                for i in 0..6 {
                    egui::ComboBox::new(format!("map-setup-choice-{i}"), "")
                        .selected_text(format!("{}", state.selected_pieces[i]))
                        .show_ui(ui, |ui| {
                            for piece in Piece::iter() {
                                for rotated in [false, true] {
                                    let choice = PieceChoice { piece, rotated };
                                    ui.selectable_value(
                                        &mut state.selected_pieces[i],
                                        choice,
                                        format!("{choice}"),
                                    );
                                }
                            }
                        });

                    if i % 2 > 0 {
                        ui.end_row();
                    }
                }
            });

            if state.are_selected_pieces_valid() {
                if ui.button("Ready").clicked() {
                    println!("start game...");
                }
            } else {
                ui.label("Select every piece once to continue");
            }
        });

        state.is_egui_hovered = ctx.wants_pointer_input();
    });

    gfx.render(&output);

    update(app, state);
}

fn update(app: &mut App, state: &mut State) {
    // Don't update if the mouse is over some egui thing
    if state.is_egui_hovered {
        return;
    }

    // Drag the drawing offset
    if app.mouse.left_is_down() {
        let delta = Vec2::from(app.mouse.position()) - state.mouse_last_frame;
        state.offset += delta;
    }

    // Remember current mouse position for next frame
    state.mouse_last_frame = app.mouse.position().into();
}
