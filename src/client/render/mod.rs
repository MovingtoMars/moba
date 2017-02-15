use piston_window::*;
use specs;

use common::{self, Player, Point};

pub mod particle;

#[derive(Clone, Copy)]
pub struct Viewport {
    // In game units
    x: f64,
    y: f64,

    scale: f64, // screen units / game units
}

impl Viewport {
    pub fn new(x: f64, y: f64, scale: f64) -> Self {
        Viewport {
            x: x,
            y: y,
            scale: scale,
        }
    }

    pub fn x_game_to_screen(&self, v: f64) -> f64 {
        (v - self.x) * self.scale
    }

    pub fn y_game_to_screen(&self, v: f64) -> f64 {
        (v - self.y) * self.scale
    }

    pub fn d_game_to_screen(&self, v: f64) -> f64 {
        v * self.scale
    }

    pub fn x_screen_to_game(&self, v: f64) -> f64 {
        (v / self.scale) + self.x
    }

    pub fn y_screen_to_game(&self, v: f64) -> f64 {
        (v / self.scale) + self.y
    }

    pub fn d_screen_to_game(&self, v: f64) -> f64 {
        v / self.scale
    }
}

pub fn render(viewport: Viewport,
              c: Context,
              g: &mut G2d,
              entity: specs::Entity,
              world: &mut specs::World) {
    let (r_component, pos_component) = (world.read::<common::Renderable>(),
                                        world.read::<common::Position>());

    if let Some(r) = r_component.get(entity) {
        let radius = viewport.d_game_to_screen(r.radius);

        let position = pos_component.get(entity).unwrap();

        ellipse(r.colour,
                [-radius, -radius, radius * 2.0, radius * 2.0],
                c.transform
                    .trans(viewport.x_game_to_screen(position.point.x),
                           viewport.y_game_to_screen(position.point.y)),
                g);
    }
}
