use common::*;
use specs::{self, Join};

pub struct ContextInner {
    events: Vec<Event>,
}

#[derive(Clone)]
pub struct Context {
    time: f64,
    inner: Arc<Mutex<ContextInner>>,
}

impl Context {
    pub fn new(time: f64) -> Self {
        Context {
            time: time,
            inner: Arc::new(Mutex::new(ContextInner { events: Vec::new() })),
        }
    }

    pub fn push_event(&self, event: Event) {
        self.inner.lock().unwrap().events.push(event);
    }

    pub fn events(&self) -> Vec<Event> {
        self.inner.lock().unwrap().events.clone()
    }
}

pub struct UpdateVelocitySystem;

impl specs::System<Context> for UpdateVelocitySystem {
    fn run(&mut self, arg: specs::RunArg, c: Context) {
        let (unitc, mut velocityc, positionc, idc, playerc) = arg.fetch(|w| {
            (w.read::<Unit>(),
             w.write::<Velocity>(),
             w.read::<Position>(),
             w.read::<EntityID>(),
             w.read::<Player>())
        });

        for (unit, velocity, player, position) in
            (&unitc, &mut velocityc, &playerc, &positionc).iter() {
            let mut speed = unit.speed;

            *velocity = match unit.target {
                Target::Nothing => Velocity { x: 0.0, y: 0.0 },
                Target::Position(p) => {
                    let dx = p.x - position.point.x;
                    let dy = p.y - position.point.y;
                    let d = (dx * dx + dy * dy).sqrt();
                    if speed * c.time > d {
                        speed = d;
                    }
                    let mut ratio = speed / d;

                    Velocity {
                        x: ratio * dx,
                        y: ratio * dy,
                    }
                }
            };
        }
    }
}

pub struct MotionSystem;

impl specs::System<Context> for MotionSystem {
    fn run(&mut self, arg: specs::RunArg, c: Context) {
        let (idc, velocityc, mut positionc) =
            arg.fetch(|w| (w.read::<EntityID>(), w.read::<Velocity>(), w.write::<Position>()));

        for (&id, velocity, mut position) in (&idc, &velocityc, &mut positionc).iter() {
            let dx = velocity.x * c.time;
            let dy = velocity.y * c.time;
            if dx.abs() < 0.1 && dy.abs() < 0.1 {
                continue;
            }
            let x = position.point.x + dx;
            let y = position.point.y + dy;

            let event = Event::EntityMove(id, Point::new(x, y));
            c.push_event(event);
        }
    }
}
