use piston_window::*;
use piston_window::character::CharacterCache;
use specs;
use gfx_device_gl::Factory;

use common;

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

pub struct Fonts {
    pub regular: Glyphs,
    pub bold: Glyphs,
}

impl Fonts {
    pub fn new(factory: Factory) -> Self {
        let regular = Glyphs::new(
            "./assets/fonts/NotoSans-unhinted/NotoSans-Regular.ttf",
            factory.clone(),
        ).unwrap();

        let bold = Glyphs::new(
            "./assets/fonts/NotoSans-unhinted/NotoSans-Bold.ttf",
            factory.clone(),
        ).unwrap();

        Fonts { regular, bold }
    }
}

pub fn render(
    viewport: Viewport,
    c: Context,
    g: &mut G2d,
    fonts: &mut Fonts,
    entity: specs::Entity,
    world: &mut specs::World,
) {
    let (r_component, pos_component, player_component) = (
        world.read::<common::Renderable>(),
        world.read::<common::Position>(),
        world.read::<common::Player>(),
    );

    if let Some(r) = r_component.get(entity) {
        let radius = viewport.d_game_to_screen(r.radius);

        let position = pos_component.get(entity).unwrap();

        let sx = viewport.x_game_to_screen(position.point.x);
        let sy = viewport.y_game_to_screen(position.point.y);

        ellipse(
            r.colour,
            [-radius, -radius, radius * 2.0, radius * 2.0],
            c.transform.trans(sx, sy),
            g,
        );

        if let Some(p) = player_component.get(entity) {
            let size = 16;
            let name = &p.name;
            let width = fonts.bold.width(size, name);

            text(
                [0.0, 0.0, 0.0, 1.0],
                size,
                name,
                &mut fonts.bold,
                c.transform.trans(sx - width / 2.0, sy - radius * 1.4),
                g,
            );
        }
    }
}
