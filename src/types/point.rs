use egui_plot::PlotPoint;

pub type X = f64;
pub type Y = f64;

pub type Point = PlotPoint;

pub const ZERO: Point = Point { x: 0.0, y: 0.0 };

pub fn rect(xy: X) -> Point {
    Point::new(xy, xy)
}

pub fn point_eq(a: &Point, b: &Point) -> bool {
    (b.x - a.x).abs() < X::EPSILON && (b.y - a.y).abs() < Y::EPSILON
}

pub fn vector(a: &Point, b: &Point) -> (X, Y) {
    (b.x - a.x, b.y - a.y)
}
