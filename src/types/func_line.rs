use super::{
    point::{self, Point, X, Y},
    vec2::Vec2,
};

pub struct FuncLine {
    k: Y,
    q: Y,
}

#[derive(Debug)]
pub enum FuncLineErr {
    IsNotLine,
    LineIsNotFunction,
}

impl FuncLine {
    pub fn new(start: &Point, end: &Point) -> Result<Self, FuncLineErr> {
        let (u1, u2) = Vec2::from(start, end).into();
        if !u1.is_finite() || !u2.is_finite() {
            return Err(FuncLineErr::IsNotLine);
        }
        let k = u2 / u1;
        if !k.is_finite() {
            return Err(FuncLineErr::LineIsNotFunction);
        }
        let q = (-k).mul_add(start.x, start.y);
        Ok(Self { k, q })
    }

    pub fn k(&self) -> Y {
        self.k
    }

    pub fn q(&self) -> Y {
        self.q
    }

    pub fn at(&self, x: X) -> Y {
        self.k.mul_add(x, self.q)
    }

    pub fn dir_vector(&self) -> Vec2 {
        Vec2::new(1.0, self.k)
    }
}
