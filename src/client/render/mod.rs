use piston_window::*;
use ecs;

use common::{MyComponents, Player, Point};

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
              entity: ecs::EntityData<MyComponents>,
              data: &mut MyComponents) {
    if let Some(r) = data.renderable.get(&entity) {
        let radius = viewport.d_game_to_screen(r.radius);

        ellipse(r.colour,
                [-radius, -radius, radius * 2.0, radius * 2.0],
                c.transform
                    .trans(viewport.x_game_to_screen(data.position[entity].x),
                           viewport.y_game_to_screen(data.position[entity].y)),
                g);
    }
}
