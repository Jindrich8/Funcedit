use std::ops::Add;

use super::point::{Point, X, Y};

#[derive(Clone, Debug)]
pub struct Vec2 {
    x: X,
    y: Y,
}

impl Vec2 {
    pub fn new(x: X, y: Y) -> Self {
        Self { x, y }
    }

    pub fn normal(&self) -> Self {
        Self {
            x: -self.y,
            y: self.x,
        }
    }

    pub fn from(a: &Point, b: &Point) -> Self {
        Self::new(b.x - a.x, b.y - a.y)
    }
}

impl Add<&Point> for &Vec2 {
    type Output = Point;
    fn add(self, other: &Point) -> Point {
        Point::new(other.x + self.x, other.y + self.y)
    }
}

impl Add<&Vec2> for &Vec2 {
    type Output = Vec2;
    fn add(self, other: &Vec2) -> Vec2 {
        Vec2::new(other.x + self.x, other.y + self.y)
    }
}

impl Into<(X, Y)> for Vec2 {
    fn into(self) -> (X, Y) {
        (self.x, self.y)
    }
}
