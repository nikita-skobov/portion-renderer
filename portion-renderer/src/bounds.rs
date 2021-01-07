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
        if rect.contains(x, y) { return true };
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

impl Rect {
    // stolen from
    // https://referencesource.microsoft.com/#System.Drawing/commonui/System/Drawing/Rectangle.cs,438
    // because im dumb and lazy
    pub fn intersection(a: Rect, b: Rect) -> Option<Rect> {
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
    pub fn contains(&self, x: u32, y: u32) -> bool {
        self.x <= x &&
        x < self.x + self.w &&
        self.y <= y &&
        y < self.y + self.h
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
}
