use std::cmp;

use super::projection::Matrix;
use super::projection::RotateMatrix;
use super::projection::RotateTranslateMatrix;
use super::projection::ComputePoint;

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
}

pub trait Contains {
    fn contains(&self, x: f32, y: f32) -> bool;
    fn contains_u32(&self, x: u32, y: u32) -> bool;
}

pub trait Intersects {
    type Obj;

    fn intersection(a: Self::Obj, b: Self::Obj) -> Option<Self::Obj>;
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

impl TiltedRect {
    pub fn prepare(&mut self) {
        self.ab_vec = vector(self.ax, self.ay, self.bx, self.by);
        self.bc_vec = vector(self.bx, self.by, self.cx, self.cy);
        self.ab_dot = dot(&self.ab_vec, &self.ab_vec);
        self.bc_dot = dot(&self.bc_vec, &self.bc_vec);
    }

    pub fn from_points(a: Point, b: Point, c: Point) -> TiltedRect {
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
        };
        t.prepare();
        t
    }
}

impl Intersects for Rect {
    type Obj = Rect;

    // stolen from
    // https://referencesource.microsoft.com/#System.Drawing/commonui/System/Drawing/Rectangle.cs,438
    // because im dumb and lazy
    fn intersection(a: Rect, b: Rect) -> Option<Rect> {
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
    fn tilted_rect_contains_works() {
        // should be approx square rotated 45degrees
        let mut t = TiltedRect {
            ax: 5.0,
            ay: 14.0,
            bx: 11.0,
            by: 20.0,
            cx: 17.0,
            cy: 14.0,
            ab_dot: 0.0,
            bc_dot: 0.0,
            ab_vec: Vector { x: 0.0, y: 0.0 },
            bc_vec: Vector { x: 0.0, y: 0.0 },
        };
        t.prepare();


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
        assert_eq!(Rect::intersection(r1, r2), None);

        // but one unit to the left of x, and
        // the intersection should be only one wide:
        let r2 = Rect {
            x: 14, y: 2,
            w: 10, h: 10,
        };
        assert_eq!(Rect::intersection(r1, r2), Some(Rect {
            x: 14, y: 2, w: 1, h: 10,
        }));

        // a rectangle entirely in another should be the smaller rect
        let r3 = Rect {
            x: 0, y: 0,
            w: 100, h: 100,
        };
        assert_eq!(Rect::intersection(r1, r3), Some(r1));
        assert_eq!(Rect::intersection(r3, r1), Some(r1));

        // can be a smaller portion in the corner somewhere
        let r4 = Rect {
            x: 7, y: 7,
            w: 100, h: 100,
        };
        assert_eq!(Rect::intersection(r4, r1), Some(Rect {
            x: 7, y: 7,
            w: 8, h: 5,
        }));
    }
}
