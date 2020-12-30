use grid::Grid;

#[derive(Default, Clone)]
pub struct GridPortion {
    active: bool,
}

#[derive(Default)]
pub struct Portioner {
    pub pix_w: u32,
    pub pix_h: u32,
    pub grid: Grid<GridPortion>,
    pub row_height: u32,
    pub col_width: u32,
}

pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

pub fn dimensions_valid(
    width: u32,
    height: u32,
    num_rows: u32,
    num_cols: u32,
) -> bool {
    if width < num_cols {
        return false;
    }
    if height < num_rows {
        return false;
    }
    if width % num_cols != 0 {
        return false;
    }
    if height % num_rows != 0 {
        return false;
    }
    true
}

impl Portioner {
    pub fn new(
        width: u32,
        height: u32,
        num_rows: u32,
        num_cols: u32,
    ) -> Portioner {
        if !dimensions_valid(width, height, num_rows, num_cols) {
            panic!("Invalid dimensions. Width/height must be larger than num_cols/num_rows, and must divide evenly.");
        }

        let row_height = height / num_rows;
        let col_width = width / num_cols;

        let mut p = Portioner::default();
        p.grid = Grid::new(num_rows as usize, num_cols as usize);
        p.row_height = row_height;
        p.col_width = col_width;
        p
    }

    pub fn take_pixel(&mut self, x: u32, y: u32) {
        let row_index = y / self.row_height;
        let col_index = x / self.col_width;
        if let Some(mut item) = self.grid.get_mut(row_index as usize, col_index as usize) {
            item.active = true;
        } else {
            println!("WARNING pixel ({}, {}) mapped to grid position ({}, {}) which doesnt exist!", x, y, col_index, row_index);
        }
    }

    /// iterates over the grid, and returns the minimum
    /// amount of contiguous active portions, and then
    /// resets the grid to not active
    pub fn flush_portions(&mut self) -> Vec<Rect> {
        let num_rows = self.grid.rows();
        let num_cols = self.grid.cols();

        // debug mode:
        if cfg!(test) || cfg!(debug_assertions) {
            println!("");
            for i in 0..num_rows {
                for item in self.grid.iter_row(i) {
                    let print = if item.active { "X" } else { "_" };
                    print!("{} ", print);
                }
                println!("");
            }
        }

        let mut out_rectangles: Vec<Rect> = vec![];
        let mut parsing_row = false;
        let mut rect_started_at = 0;
        for i in 0..num_rows {
            let mut j = 0;
            for item in self.grid.iter_row_mut(i) {
                if item.active && ! parsing_row {
                    parsing_row = true;
                    rect_started_at = j;
                }

                if !item.active && parsing_row {
                    // we reached the end of contiguous row segments
                    // mark the end
                    parsing_row = false;
                    let this_rect_width = j - rect_started_at;
                    let this_rect = Rect {
                        x: rect_started_at,
                        y: i as u32,
                        w: this_rect_width,
                        h: 1,
                    };
                    rect_started_at = 0;

                    let mut should_add_this_rect = true;
                    for last_rect in out_rectangles.iter_mut().rev() {
                        // we keep iterating over the rectangles that were
                        // above this current row. as soon as we find a rectangle
                        // that doesnt touch/reach this current row, we can
                        // stop iterating because that means there was a gap between
                        // our row and all previous rectangles
                        let last_rect_touches_this_rect = last_rect.y + last_rect.h == i as u32;
                        if !last_rect_touches_this_rect {
                            break;
                        }

                        // if the previous rectangle DOES reach this row,
                        // then we also check if the dimensions are the same, if they match
                        // then we simply extend the height of the previous rectangle by 1
                        if last_rect.x == this_rect.x && last_rect.w == this_rect.w {
                            last_rect.h += 1;
                            should_add_this_rect = false;
                            break;
                        }
                    }

                    if should_add_this_rect {
                        out_rectangles.push(this_rect);
                    }
                }

                item.active = false;
                j += 1;
            }

            if parsing_row {
                // if we reached the end of the row, we also mark that this is
                // the end of this rectangle segment
                let this_rect_width = num_cols as u32 - rect_started_at;
                let this_rect = Rect {
                    x: rect_started_at,
                    y: i as u32,
                    w: this_rect_width,
                    h: 1,
                };

                let mut should_add_this_rect = true;
                for last_rect in out_rectangles.iter_mut().rev() {
                    // we keep iterating over the rectangles that were
                    // above this current row. as soon as we find a rectangle
                    // that doesnt touch/reach this current row, we can
                    // stop iterating because that means there was a gap between
                    // our row and all previous rectangles
                    let last_rect_touches_this_rect = last_rect.y + last_rect.h == i as u32;
                    if !last_rect_touches_this_rect {
                        break;
                    }

                    // if the previous rectangle DOES reach this row,
                    // then we also check if the dimensions are the same, if they match
                    // then we simply extend the height of the previous rectangle by 1
                    if last_rect.x == this_rect.x && last_rect.w == this_rect.w {
                        last_rect.h += 1;
                        should_add_this_rect = false;
                        break;
                    }
                }

                if should_add_this_rect {
                    out_rectangles.push(this_rect);
                }
            }

            rect_started_at = 0;
            parsing_row = false;
        }

        out_rectangles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_WIDTH: u32 = 800;
    const TEST_HEIGHT: u32 = 600;

    #[test]
    fn divides_properly() {
        let p = Portioner::new(
            TEST_WIDTH, TEST_HEIGHT, 4, 4,
        );
        // should make a 4x4 grid
        let num_grid_items = p.grid.rows() * p.grid.cols();
        assert_eq!(num_grid_items, 16);
    }

    #[test]
    #[should_panic(expected = "dimensions")]
    fn should_panic_if_invalid_dims() {
        let _ = Portioner::new(
            TEST_WIDTH, TEST_HEIGHT, 100000, 1000000
        );
    }

    #[test]
    fn take_pixel_works_1() {
        let mut p = Portioner::new(
            10, 10, 10, 10
        );
        for item in p.grid.iter() {
            assert!(!item.active);
        }
        p.take_pixel(0, 0);
        let thing = p.grid.get(0, 0).unwrap();
        assert!(thing.active);

        let non_active = p.grid.get(0, 1).unwrap();
        assert!(!non_active.active);

        p.take_pixel(9, 9);
        let thing = p.grid.get(9, 9).unwrap();
        assert!(thing.active);
    }

    #[test]
    fn flush_portions_gives_minimal_rectangles() {
        // simple square, should be 1 rect
        let mut p = Portioner::new(
            10, 10, 10, 10
        );
        p.take_pixel(0, 0);
        p.take_pixel(0, 1);
        p.take_pixel(1, 0);
        p.take_pixel(1, 1);
        let portion_vec = p.flush_portions();
        assert_eq!(portion_vec.len(), 1);

        // 1 row skipped, should be 2 seperate rects
        let mut p = Portioner::new(
            10, 10, 10, 10
        );
        p.take_pixel(0, 0);
        p.take_pixel(1, 0);
        p.take_pixel(0, 2);
        p.take_pixel(1, 2);
        let portion_vec = p.flush_portions();
        assert_eq!(portion_vec.len(), 2);

        // entire grid, should be 1 rect
        let mut p = Portioner::new(
            10, 10, 10, 10
        );
        for i in 0..10 {
            for j in 0..10 {
                p.take_pixel(i, j);
            }
        }
        let portion_vec = p.flush_portions();
        assert_eq!(portion_vec.len(), 1);

        // entire column down the middle
        let mut p = Portioner::new(
            10, 10, 10, 10
        );
        for i in 0..10 {
            for j in 3..7 {
                p.take_pixel(j, i);
            }
        }
        let portion_vec = p.flush_portions();
        assert_eq!(portion_vec.len(), 1);

        // medium challenge: 2 columns seperated by gap
        let mut p = Portioner::new(
            10, 10, 10, 10
        );
        for i in 0..10 {
            p.take_pixel(0, i);
            p.take_pixel(1, i);
            p.take_pixel(8, i);
            p.take_pixel(9, i);
        }
        let portion_vec = p.flush_portions();
        assert_eq!(portion_vec.len(), 2);

        // final challenge: 2 rows, first full
        // second less than full should be 2 rects
        // then theres a square in the middle + 1
        // and bottom right two corners + 2
        // total should be 5
        let mut p = Portioner::new(
            10, 10, 10, 10
        );
        for i in 0..10 {
            p.take_pixel(i, 0);
            if i != 9 {
                p.take_pixel(i, 1);
            }
            if i >= 3 && i < 7 {
                p.take_pixel(i, 4);
                p.take_pixel(i, 5);
                p.take_pixel(i, 6);
            }
            if i >= 7 {
                p.take_pixel(0, i);
                p.take_pixel(1, i);
                p.take_pixel(2, i);
                p.take_pixel(7, i);
                p.take_pixel(8, i);
                p.take_pixel(9, i);
            }
        }
        let portion_vec = p.flush_portions();
        assert_eq!(portion_vec.len(), 5);
    }

    #[test]
    fn flush_portions_resets_the_grid() {
        // simple square, should be 1 rect
        let mut p = Portioner::new(
            10, 10, 10, 10
        );
        p.take_pixel(0, 0);
        p.take_pixel(0, 1);
        p.take_pixel(1, 0);
        p.take_pixel(1, 1);
        let portion_vec = p.flush_portions();
        assert_eq!(portion_vec.len(), 1);
        let portion_vec = p.flush_portions();
        assert!(portion_vec.is_empty());
        for griditem in p.grid.iter() {
            assert!(!griditem.active);
        }
    }
}
