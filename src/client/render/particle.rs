use super::Viewport;

use piston_window::*;

pub trait Particle {
    fn render(&mut self, viewport: Viewport, Context, &mut G2d);
    fn update(&mut self, world_time: f64);
    fn should_remove(&self) -> bool;
}

pub struct RightClick {
    x: f64,
    y: f64,
    time: f64,
}

impl RightClick {
    pub fn new(x: f64, y: f64) -> Self {
        RightClick {
            x: x,
            y: y,
            time: 0.3,
        }
    }
}

impl Particle for RightClick {
    fn update(&mut self, t: f64) {
        self.time -= t;
    }

    fn should_remove(&self) -> bool {
        self.time < 0.0
    }

    fn render(&mut self, viewport: Viewport, c: Context, g: &mut G2d) {
        let radius = 10.0 * self.time / 0.4;
        ellipse([0.0, 0.0, 1.0, 1.0],
                [-radius, -radius, radius * 2.0, radius * 2.0],
                c.transform
                    .trans(viewport.x_game_to_screen(self.x),
                           viewport.y_game_to_screen(self.y)),
                g);
    }
}
