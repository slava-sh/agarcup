use std::ops::{Mul, Add, Div, Sub};

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    pub fn from_polar(r: f64, angle: f64) -> Point {
        Point::new(r * angle.cos(), r * angle.sin())
    }

    pub fn zero() -> Point {
        Point::new(0.0, 0.0)
    }

    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }

    pub fn length(self) -> f64 {
        self.x.hypot(self.y)
    }

    pub fn with_length(self, new_length: f64) -> Point {
        let current_length = self.length();
        if current_length == 0.0 {
            Point::zero()
        } else {
            self * (new_length / current_length)
        }
    }

    pub fn unit(self) -> Point {
        self.with_length(1.0)
    }
}

impl Mul<f64> for Point {
    type Output = Point;
    fn mul(self, k: f64) -> Point {
        Point::new(self.x * k, self.y * k)
    }
}

impl Div<f64> for Point {
    type Output = Point;
    fn div(self, k: f64) -> Point {
        Point::new(self.x / k, self.y / k)
    }
}

pub trait HasPoint {
    fn point(&self) -> Point;

    fn x(&self) -> f64 {
        self.point().x
    }

    fn y(&self) -> f64 {
        self.point().y
    }
}

impl HasPoint for Point {
    fn point(&self) -> Point {
        *self
    }
}

impl Point {
    pub fn dist<Other: HasPoint>(self, other: Other) -> f64 {
        (self.x - other.x()).hypot(self.y - other.y())
    }

    pub fn qdist<Other: HasPoint>(self, other: Other) -> f64 {
        (self.x - other.x()).powi(2) + (self.y - other.y()).powi(2)
    }
}

impl<Other: HasPoint> Add<Other> for Point {
    type Output = Point;
    fn add(self, other: Other) -> Point {
        Point::new(self.x + other.x(), self.y + other.y())
    }
}

impl<Other: HasPoint> Sub<Other> for Point {
    type Output = Point;
    fn sub(self, other: Other) -> Point {
        Point::new(self.x - other.x(), self.y - other.y())
    }
}
