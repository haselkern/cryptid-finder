#![allow(unused, dead_code)] // TODO Remove once done.

mod buildingmap;
mod model;

use crate::model::*;
use std::{collections::HashMap, f32::consts::PI};

use hexx::{HexLayout, HexOrientation};
use notan::{
    draw::{CreateDraw, DrawConfig, DrawImages, DrawShapes, DrawTransform},
    egui::{EguiConfig, EguiPluginSugar},
    math::{Mat3, Vec2},
    prelude::*,
};
use strum::IntoEnumIterator;

#[derive(AppState)]
struct State {
    // Radius of the tiles to draw
    tile_radius: f32,
    // Offset to draw the tiles at. Used for dragging with mouse.
    offset: Vec2,
    icons: HashMap<Terrain, Texture>,
    is_egui_hovered: bool,
    mouse_last_frame: Vec2,
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
            mouse_last_frame: Vec2::ZERO,
            sub: Default::default(),
        }
    }
}

enum SubState {
    BuildingMap(buildingmap::SubState),
}

impl Default for SubState {
    fn default() -> Self {
        Self::BuildingMap(buildingmap::SubState::default())
    }
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
    if !state.is_egui_hovered {
        if let Event::MouseWheel { delta_y, .. } = event {
            state.tile_radius = (state.tile_radius + delta_y).clamp(8.0, 1024.0);
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

    let tiles = match &state.sub {
        SubState::BuildingMap(sub) => sub.tiles(),
    };

    for tile in tiles {
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
        match &mut state.sub {
            SubState::BuildingMap(sub) => sub.gui(ctx),
        }

        state.is_egui_hovered = ctx.wants_pointer_input();
    });

    gfx.render(&output);

    // Perform the update now. We now know whether we should process mouse events,
    // or if egui already handled them.
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
