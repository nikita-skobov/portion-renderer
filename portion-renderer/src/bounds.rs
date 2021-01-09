use std::cmp;
use super::Matrix;

pub static EMPTY_RECT: Rect = Rect { x: 0, y: 0, w: 0, h: 0 };

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Copy, Clone)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
}

/// point B must be 'between'
/// point A and C. ie, you should not be
/// able to draw a direct line between A and C, but
/// rather you have to cross B first.
pub struct TiltedRect {
    pub ax: f32,
    pub ay: f32,
    pub bx: f32,
    pub by: f32,
    pub cx: f32,
    pub cy: f32,
    pub ab_vec: Vector,
    pub ab_dot: f32,
    pub bc_vec: Vector,
    pub bc_dot: f32,
    pub bounding_rect: Rect,
}

pub trait Contains {
    fn contains(&self, x: f32, y: f32) -> bool;
    fn contains_u32(&self, x: u32, y: u32) -> bool;
}

pub trait GetRectangularBounds {
    fn get_bounds(&self) -> Rect;
}

pub trait Intersects {
    fn intersection<C: GetRectangularBounds>(&self, b: C) -> Option<Rect>;
}

#[inline(always)]
pub fn vector(x1: f32, y1: f32, x2: f32, y2: f32) -> Vector {
    Vector {
        x: x2 - x1,
        y: y2 - y1,
    }
}

#[inline(always)]
pub fn dot(u: &Vector, v: &Vector) -> f32 {
    u.x * v.x + u.y * v.y
}

pub fn should_skip_point(skip_regions: &Vec<Rect>, x: u32, y: u32) -> bool {
    for rect in skip_regions {
        if rect.contains_u32(x, y) { return true };
    }
    false
}

pub fn sorted_values(a: &Point, b: &Point, c: &Point) -> [[f32; 3]; 2] {
    let mut x = [a.x, b.x, c.x];
    let mut y = [a.y, b.y, c.y];

    x.sort_by(|p1, p2| p1.partial_cmp(p2).unwrap());
    y.sort_by(|p1, p2| p1.partial_cmp(p2).unwrap());

    return [ x, y ];
}

impl Point {
    pub fn transform_by(&mut self, matrix: &Matrix) {
        let (x, y) = matrix.mul_point(self.x, self.y);
        self.x = x;
        self.y = y;
    }
}

impl TiltedRect {
    /// given an original, non-rotated rectangle. create a tilted rect bounds
    /// via the original bounds and the desired transformation matrix.
    pub fn from_bounds_and_matrix(bounds: Rect, matrix: Matrix) -> TiltedRect {
        let x = bounds.x as f32;
        let y = bounds.y as f32;
        let max_x = x + bounds.w as f32 - 1.0;
        let max_y = y + bounds.h as f32 - 1.0;
        let mut a = Point { x, y };
        let mut b = Point { x: max_x, y };
        let mut c = Point { x: max_x, y: max_y };
        a.transform_by(&matrix);
        b.transform_by(&matrix);
        c.transform_by(&matrix);
        TiltedRect::from_points(a, b, c)
    }

    pub fn prepare(&mut self) {
        self.ab_vec = vector(self.ax, self.ay, self.bx, self.by);
        self.bc_vec = vector(self.bx, self.by, self.cx, self.cy);
        self.ab_dot = dot(&self.ab_vec, &self.ab_vec);
        self.bc_dot = dot(&self.bc_vec, &self.bc_vec);
    }

    pub fn from_points(a: Point, b: Point, c: Point) -> TiltedRect {
        let [sorted_x, sorted_y] = sorted_values(&a, &b, &c);

        let x = sorted_x[0].ceil() as u32;
        let y = sorted_y[0].ceil() as u32;
        let w = (sorted_x[2] - sorted_x[1] + sorted_x[1] - sorted_x[0]).ceil() as u32;
        let h = (sorted_y[2] - sorted_y[0] + sorted_y[1] - sorted_y[0]).ceil() as u32;

        let mut t = TiltedRect {
            ax: a.x,
            ay: a.y,
            bx: b.x,
            by: b.y,
            cx: c.x,
            cy: c.y,
            ab_vec: Vector { x: 0.0, y: 0.0, },
            bc_vec: Vector { x: 0.0, y: 0.0, },
            ab_dot: 0.0,
            bc_dot: 0.0,
            bounding_rect: Rect { x, y, w, h },
        };
        t.prepare();
        t
    }
}

impl Intersects for TiltedRect {
    /// too lazy right now to figure out a good intersection
    /// algorithm for tilted rectangles... just going to
    /// use the rectangular outer bounds
    fn intersection<C: GetRectangularBounds>(&self, b: C) -> Option<Rect> {
        self.bounding_rect.intersection(b.get_bounds())
    }
}

impl GetRectangularBounds for Rect {
    fn get_bounds(&self) -> Rect {
        *self
    }
}

impl GetRectangularBounds for TiltedRect {
    fn get_bounds(&self) -> Rect {
        self.bounding_rect
    }
}

impl Intersects for Rect {
    // stolen from
    // https://referencesource.microsoft.com/#System.Drawing/commonui/System/Drawing/Rectangle.cs,438
    // because im dumb and lazy
    fn intersection<C: GetRectangularBounds>(&self, b: C) -> Option<Rect> {
        let b = b.get_bounds();
        let a = self;
        let x1 = cmp::max(a.x, b.x);
        let x2 = cmp::min(a.x + a.w, b.x + b.w);
        let y1 = cmp::max(a.y, b.y);
        let y2 = cmp::min(a.y + a.h, b.y + b.h);

        if x2 > x1 && y2 > y1 {
            Some(Rect { x: x1, y: y1, w: x2 - x1, h: y2 - y1 })
        } else {
            None
        }
    }
}

impl Contains for Rect {
    #[inline(always)]
    fn contains(&self, x: f32, y: f32) -> bool {
        self.contains_u32(x as u32, y as u32)
    }

    #[inline(always)]
    fn contains_u32(&self, x: u32, y: u32) -> bool {
        // this is actually faster if its not inlined
        // self.x <= x && self.y <= y && x < self.x + self.w && y < self.y + self.h

        // but if its inlined, then this is faster:
        let dum_arr = [
            self.x <= x,
            self.y <= y,
            x < self.x + self.w,
            y < self.y + self.h,
        ];
        dum_arr.iter().all(|p| *p == true)
    }
}

impl Contains for TiltedRect {
    #[inline(always)]
    fn contains(&self, x: f32, y: f32) -> bool {
        let ab = self.ab_vec;
        let bc = self.bc_vec;
        let am = vector(self.ax, self.ay, x, y);
        let bm = vector(self.bx, self.by, x, y);

        let [
            dot_abab,
            dot_bcbc,
            dot_abam,
            dot_bcbm,
        ] = [
            self.ab_dot,
            self.bc_dot,
            dot(&ab, &am),
            dot(&bc, &bm),
        ];

        let dum_arr = [
            dot_abam >= 0.0,
            dot_bcbm >= 0.0,
            dot_abam <= dot_abab,
            dot_bcbm <= dot_bcbc
        ];
        dum_arr.iter().all(|p| *p == true)
    }

    #[inline(always)]
    fn contains_u32(&self, x: u32, y: u32) -> bool {
        self.contains(x as f32, y as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilted_rect_intersection_works() {
        // should be approx square rotated 45degrees
        let t = TiltedRect::from_points(
            Point { x: 0.0, y: 5.0 },
            Point { x: 5.0, y: 0.0 },
            Point { x: 6.0, y: 1.0 },
        );
        
        let r = Rect { x: 1, y: 1, w: 1, h: 1 };
        assert_eq!(t.intersection(r), Some(r));
        assert_eq!(r.intersection(t), Some(r));
    }

    #[test]
    fn tilted_rect_bounds_are_correct() {
        // should be approx square rotated 45degrees
        let t = TiltedRect::from_points(
            Point { x: 0.0, y: 5.0 },
            Point { x: 5.0, y: 0.0 },
            Point { x: 6.0, y: 1.0 },
        );

        assert_eq!(t.bounding_rect, Rect {
            x: 0, y: 0,
            w: 6, h: 6,
        });

        let t = TiltedRect::from_points(
            Point { x: 1.0, y: 8.0 },
            Point { x: 4.0, y: 2.0 },
            Point { x: 4.8, y: 2.4 },
        );

        assert_eq!(t.bounding_rect, Rect {
            x: 1, y: 2,
            w: 4, h: 7,
        });

        // still works for regular rectangles:
        let t = TiltedRect::from_points(
            Point { x: 1.0, y: 5.0 },
            Point { x: 1.0, y: 7.0 },
            Point { x: 5.0, y: 5.0 },
        );
        assert_eq!(t.bounding_rect, Rect {
            x: 1, y: 5,
            w: 4, h: 2,
        });
    }

    #[test]
    fn tilted_rect_contains_works() {
        // should be approx square rotated 45degrees
        let t = TiltedRect::from_points(
            Point { x: 5.0, y: 14.0 },
            Point { x: 11.0, y: 20.0 },
            Point { x: 17.0, y: 14.0 },
        );

        // exactly in the center should contain
        assert!(t.contains(11.0, 14.0));

        // // below 8 should not
        assert!(t.contains(11.0, 9.0));
        assert!(t.contains(11.0, 8.0));
        assert!(! t.contains(11.0, 7.0));

        // widest is at y = 14?
        assert!(!t.contains(4.0, 14.0));
        assert!(t.contains(5.0, 14.0));
        assert!(t.contains(17.0, 14.0));
        assert!(! t.contains(18.0, 14.0));

        // on the upper left edge:
        assert!(t.contains(8.0, 17.0));
        // but one left or one above should fail
        assert!(! t.contains(7.0, 17.0));
        assert!(! t.contains(8.0, 18.0));

        // on the bottom right edge:
        assert!(t.contains(15.0, 12.0));
        // but one right or one below should fail
        assert!(! t.contains(16.0, 12.0));
        assert!(! t.contains(15.0, 11.0));
    }

    #[test]
    fn tilted_rect_contains_works_regardless_of_abc() {
        // shouldnt matter which points you choose for ABC as long
        // as B is between A and C.
        // this test is the same as the 'tilted_rect_contains_works' test
        // but with different points for A B C

        let left = Point { x: 5.0, y: 14.0 };
        let top = Point { x: 11.0, y: 8.0 };
        let right = Point { x: 17.0, y: 14.0 };
        let bottom = Point { x: 11.0, y: 20.0 };

        let combinations = vec![
            (left, bottom, right),
            (bottom, right, top),
            (right, top, left),
            (top, left, bottom)
        ];

        for (a, b, c) in combinations {
            // should be approx square rotated 45degrees
            let t = TiltedRect::from_points(a, b, c);
            // exactly in the center should contain
            assert!(t.contains(11.0, 14.0));

            // // below 8 should not
            assert!(t.contains(11.0, 9.0));
            assert!(t.contains(11.0, 8.0));
            assert!(! t.contains(11.0, 7.0));

            // widest is at y = 14?
            assert!(!t.contains(4.0, 14.0));
            assert!(t.contains(5.0, 14.0));
            assert!(t.contains(17.0, 14.0));
            assert!(! t.contains(18.0, 14.0));

            // on the upper left edge:
            assert!(t.contains(8.0, 17.0));
            // but one left or one above should fail
            assert!(! t.contains(7.0, 17.0));
            assert!(! t.contains(8.0, 18.0));

            // on the bottom right edge:
            assert!(t.contains(15.0, 12.0));
            // but one right or one below should fail
            assert!(! t.contains(16.0, 12.0));
            assert!(! t.contains(15.0, 11.0));
        }
    }

    #[test]
    fn rext_contains_works() {
        let r = Rect {
            x: 5,
            y: 2,
            w: 10,
            h: 10,
        };

        // somewhere in middle
        assert!(r.contains_u32(7, 7));

        // all corners:
        assert!(r.contains_u32(5, 2));
        assert!(r.contains_u32(14, 2));
        assert!(r.contains_u32(14, 11));
        assert!(r.contains_u32(5, 11));

        // one off from each corner should not contain
        assert!(! r.contains_u32(4, 2));
        assert!(! r.contains_u32(5, 1));
        assert!(! r.contains_u32(15, 2));
        assert!(! r.contains_u32(14, 1));
        assert!(! r.contains_u32(15, 11));
        assert!(! r.contains_u32(14, 12));
        assert!(! r.contains_u32(4, 11));
        assert!(! r.contains_u32(5, 12));
    }

    #[test]
    fn rect_intersection_works() {
        let r1 = Rect {
            x: 5, y: 2,
            w: 10, h: 10,
        };
        let r2 = Rect {
            x: 15, y: 2,
            w: 10, h: 10,
        };
        // adjacent rects should not intersect
        assert_eq!(r1.intersection(r2), None);

        // but one unit to the left of x, and
        // the intersection should be only one wide:
        let r2 = Rect {
            x: 14, y: 2,
            w: 10, h: 10,
        };
        assert_eq!(r1.intersection(r2), Some(Rect {
            x: 14, y: 2, w: 1, h: 10,
        }));

        // a rectangle entirely in another should be the smaller rect
        let r3 = Rect {
            x: 0, y: 0,
            w: 100, h: 100,
        };
        assert_eq!(r1.intersection(r3), Some(r1));
        assert_eq!(r3.intersection(r1), Some(r1));

        // can be a smaller portion in the corner somewhere
        let r4 = Rect {
            x: 7, y: 7,
            w: 100, h: 100,
        };
        assert_eq!(r4.intersection(r1), Some(Rect {
            x: 7, y: 7,
            w: 8, h: 5,
        }));
    }
}
