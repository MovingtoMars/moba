use std::ops::{Sub, Mul};
use na::{Point2, Vector2};


#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Point { x: x, y: y }
    }

    pub fn distance_to(self: Point, p: Point) -> f64 {
        (p - self).norm()
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
}

impl Vector {
    pub fn norm(self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn with_norm(mut self, norm: f64) -> Self {
        assert!(norm >= 0.0);
        let ratio = norm / self.norm();
        self.x *= ratio;
        self.y *= ratio;
        self
    }
}

impl Mul<f64> for Vector {
    type Output = Vector;

    fn mul(self, right: f64) -> Self::Output {
        Vector {
            x: self.x * right,
            y: self.y * right,
        }
    }
}

impl Sub<Point> for Point {
    type Output = Vector;

    fn sub(self, right: Point) -> Self::Output {
        Vector {
            x: self.x - right.x,
            y: self.y - right.y,
        }
    }
}

impl From<Point> for Point2<f64> {
    fn from(p: Point) -> Self {
        Point2::new(p.x, p.y)
    }
}

impl From<Point2<f64>> for Point {
    fn from(p: Point2<f64>) -> Self {
        Point::new(p.x, p.y)
    }
}

impl From<Point> for Vector2<f64> {
    fn from(p: Point) -> Self {
        Vector2::new(p.x, p.y)
    }
}

impl From<Vector2<f64>> for Point {
    fn from(p: Vector2<f64>) -> Self {
        Point::new(p.x, p.y)
    }
}
