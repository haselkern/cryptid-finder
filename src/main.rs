mod model;
mod substate;

use crate::model::*;
use std::{collections::HashMap, f32::consts::PI};

use hexx::{Hex, HexLayout, HexOrientation};
use notan::{
    draw::{CreateDraw, DrawConfig, DrawImages, DrawShapes, DrawTransform},
    egui::{self, EguiConfig, EguiPluginSugar, Frame, RichText, ScrollArea, Style},
    math::{Mat3, Vec2},
    prelude::*,
};
use strum::IntoEnumIterator;
use substate::{Common, SubState};

pub const LAYOUT_SPACE: f32 = 16.0;
pub const START_MAXIMIZED: bool = cfg!(target_family = "wasm");

#[derive(AppState)]
struct State {
    /// Radius of the tiles to draw
    tile_radius: f32,
    /// Offset to draw the tiles at. Used for dragging with mouse.
    offset: Vec2,
    icons: HashMap<Terrain, Texture>,
    is_egui_hovered: bool,
    dragging: Dragging,
    sub: SubState,
}

impl State {
    fn new(gfx: &mut Graphics) -> Self {
        let icons = load_icons(gfx);

        Self {
            tile_radius: 64.0,
            icons,
            is_egui_hovered: false,
            offset: Vec2::ZERO,
            dragging: Dragging::None,
            sub: Default::default(),
        }
    }

    /// True if the structures are supposed to be draggable in this substate.
    fn are_structures_draggable(&self) -> bool {
        matches!(self.sub, SubState::PlacingStructures(_))
    }
}

/// Possible dragging modes.
#[derive(Debug, Clone, Copy)]
enum Dragging {
    /// No dragging active.
    None,
    /// The offset i.e. the screen if being dragged.
    Offset { mouse_last_frame: Vec2 },
    /// A structure (currently on the tile at the Hex) is being dragged to another tile.
    Structure(Hex),
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
        .add_config(
            WindowConfig::new()
                .resizable(true)
                .maximized(START_MAXIMIZED)
                .title("Cryptid Finder"),
        )
        .build()
}

fn event(state: &mut State, event: Event) {
    if !state.is_egui_hovered {
        if let Event::MouseWheel { delta_y, .. } = event {
            state.tile_radius = (state.tile_radius + delta_y * 0.1).clamp(8.0, 1024.0);
        }
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

    for tile in state.sub.tiles() {
        let pos = layout.hex_to_world_pos(tile.position);

        let scale = if tile.small {
            Mat3::from_scale(Vec2::splat(0.7))
        } else {
            Mat3::IDENTITY
        };
        let alpha = if tile.small { 0.6 } else { 1.0 };

        draw.transform().push(Mat3::from_translation(pos) * scale);

        // Draw flat topped hex
        {
            draw.transform().push(Mat3::from_rotation_z(PI / 6.0));

            draw.polygon(6, state.tile_radius)
                .color(tile.terrain.into())
                .alpha(alpha);

            if let Some(animal) = tile.animal {
                let color = match animal {
                    Animal::Bear => Color::BLACK,
                    Animal::Cougar => Color::from_bytes(220, 25, 11, 255),
                };

                draw.polygon(6, state.tile_radius * 0.9)
                    .stroke(stroke_width)
                    .stroke_color(color)
                    .alpha(alpha);
            }

            draw.transform().pop();
        }

        // Draw icon for terrain
        if !tile.small {
            let tex = state.icons.get(&tile.terrain).unwrap();
            let scale = state.tile_radius * 0.015;
            let size = Vec2::from(tex.size());
            draw.transform()
                .push(Mat3::from_scale(Vec2::splat(scale)) * Mat3::from_translation(size * -0.5));
            draw.image(tex).alpha(0.3);
            draw.transform().pop();
        }

        // Draw structure
        if let Some(building) = tile.structure {
            let color = building.color.into();
            let sides = match building.kind {
                StructureKind::Shack => 3,
                StructureKind::Stone => 8,
            };

            draw.polygon(sides, state.tile_radius * 0.5)
                .color(color)
                .rotate(PI);
            draw.polygon(sides, state.tile_radius * 0.5)
                .stroke(stroke_width)
                .stroke_color(Color::BLACK)
                .rotate(PI);
        }

        // Draw answers in a little circle.
        for (i, (&player_id, &answer)) in tile.answers.iter().enumerate() {
            let player = state.sub.players().get(player_id);
            let angle = i as f32;
            let radius = state.tile_radius * 0.6;
            let x = angle.cos() * radius;
            let y = angle.sin() * radius;
            let circle_radius = state.tile_radius * 0.2;
            let box_width = state.tile_radius * 0.4;
            let outline_stroke = (stroke_width * 0.5).max(1.0);
            match answer {
                Answer::Unknown => (),
                Answer::Yes => {
                    draw.circle(circle_radius)
                        .color(player.color.into())
                        .position(x, y);
                    draw.circle(circle_radius)
                        .stroke_color(Color::BLACK)
                        .stroke(outline_stroke)
                        .position(x, y);
                }
                Answer::No => {
                    draw.rect(
                        (x - box_width * 0.5, y - box_width * 0.5),
                        (box_width, box_width),
                    )
                    .color(player.color.into());
                    draw.rect(
                        (x - box_width * 0.5, y - box_width * 0.5),
                        (box_width, box_width),
                    )
                    .stroke_color(Color::BLACK)
                    .stroke(outline_stroke);
                }
            }
        }

        draw.transform().pop();
    }

    // This tile might be highlighted
    for highlight in state.sub.highlights() {
        let position = layout.hex_to_world_pos(highlight);
        draw.transform().push(Mat3::from_translation(position));
        draw.polygon(6, state.tile_radius * 0.8)
            .stroke(stroke_width)
            .stroke_color(Color::YELLOW)
            .rotate(app.timer.time_since_init());
        draw.transform().pop();
    }

    gfx.render(&draw);

    let mut switch_state = false;

    let output = plugins.egui(|ctx| {
        let frame = Frame::side_top_panel(&Style::default()).inner_margin(LAYOUT_SPACE);
        egui::SidePanel::left("sidepanel")
            .resizable(true)
            .frame(frame)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Cryptid Finder");
                    ui.label(RichText::new("by haselkern").weak());
                    ui.add_space(LAYOUT_SPACE);

                    switch_state = state.sub.gui(ui);
                });
            });

        if switch_state {
            ctx.memory().reset_areas();
        }

        state.is_egui_hovered = ctx.is_pointer_over_area() || ctx.is_using_pointer();
    });

    gfx.render(&output);

    if switch_state {
        match &state.sub {
            SubState::BuildingMap(sub) => state.sub = SubState::PlacingStructures(sub.into()),
            SubState::PlacingStructures(sub) => state.sub = SubState::TryingClues(sub.into()),
            other => {
                panic!("{other:?} wanted to switch states, but I don't know how :( This is a bug.")
            }
        };
    }

    // Perform the update now. We now know whether we should process mouse events,
    // or if egui already handled them.
    update(app, state, &layout);
}

fn update(app: &mut App, state: &mut State, layout: &HexLayout) {
    let mouse = Vec2::from(app.mouse.position());
    let mouse_hex = layout.world_pos_to_hex(mouse);

    if app.mouse.left_was_released() && !state.is_egui_hovered {
        state.sub.click(mouse_hex);
    }

    if app.mouse.left_is_down() {
        match state.dragging {
            Dragging::None => {
                // Don't start dragging anything when the mouse is over egui
                if state.is_egui_hovered {
                    return;
                }

                // Start dragging a structure (if that is allowed) or the screen.
                let over_tile = state.sub.tiles().iter().find(|t| t.position == mouse_hex);
                let has_structure = over_tile.map(|t| t.structure.is_some()).unwrap_or(false);

                if has_structure && state.are_structures_draggable() {
                    state.dragging = Dragging::Structure(mouse_hex);
                } else {
                    state.dragging = Dragging::Offset {
                        mouse_last_frame: app.mouse.position().into(),
                    };
                }
            }
            Dragging::Offset { mouse_last_frame } => {
                let delta = mouse - mouse_last_frame;
                state.offset += delta;
                state.dragging = Dragging::Offset {
                    mouse_last_frame: mouse,
                };
            }
            Dragging::Structure(at) => {
                // Check if the hex under the mouse has space for the structure.
                // Move the structure (currently "at" another hex) to there.
                let mouse_hex = layout.world_pos_to_hex(mouse);
                let tiles = state.sub.tiles_mut();

                let Some(to) = tiles.iter().position(|t| t.position == mouse_hex) else {
                    // No tile under mouse.
                    return;
                };

                if tiles[to].structure.is_some() {
                    // Tile under mouse already has a structure.
                    return;
                }

                let from = tiles
                    .iter()
                    .position(|t| t.position == at)
                    .expect("The map changed drastically. This should not happen.");

                tiles[to].structure = tiles[from].structure.take();
                state.dragging = Dragging::Structure(mouse_hex);
            }
        }
    } else {
        state.dragging = Dragging::None;
    }
}
