use std::cmp;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

pub fn should_skip_point(skip_regions: &Vec<Rect>, x: u32, y: u32) -> bool {
    for rect in skip_regions {
        if rect.contains(x, y) { return true };
    }
    false
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
