use grid::Grid;
use std::{ops::Index, cmp};

macro_rules! get_red_index {
    ($x:expr, $y:expr, $w:expr, $indices_per_pixel:expr) => {
        $y * ($w * $indices_per_pixel) + ($x * $indices_per_pixel)
    };
}

pub const PIXEL_BLACK: RgbaPixel = RgbaPixel { r: 0, g: 0, b: 0, a: 0 };
pub const PIXEL_RED: RgbaPixel = RgbaPixel { r: 255, g: 0, b: 0, a: 0 };
pub const PIXEL_GREEN: RgbaPixel = RgbaPixel { r: 0, g: 255, b: 0, a: 0 };
pub const PIXEL_BLUE: RgbaPixel = RgbaPixel { r: 0, g: 0, b: 255, a: 0 };

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

pub struct PortionRenderer {
    pixel_buffer: Vec<u8>,
    clear_buffer: Vec<u8>,
    portioner: Portioner,

    width: u32,
    height: u32,
    indices_per_pixel: u32, // probably only 3 or 4

    textures: Vec<Vec<u8>>,
    layers: Vec<ManagedLayer>,
    objects: Vec<Object>,

    // TODO: need to know what
    // order the pixels are in
    // pixel_format: PixelFormatEnum
}

pub struct ManagedLayer {
    index: u32,
    /// a vector of indices to the objects that exist on this layer
    /// these objects can be accessed by PortionRenderer.objects[ManagedLayer.objects[...]]
    objects: Vec<usize>,
    /// a vector of indices to the objects on this layer that need updating
    /// these objects can be accessed by
    /// PortionRenderer.objects[ManagedLayer.objects[ManagedLayer.updates[...]]]
    updates: Vec<usize>,
}

pub struct Object {
    /// most objects will have a reference
    /// to a vector of their texture pixels
    texture_index: usize,
    /// some objects might choose to be a single color,
    /// in which case they will be rendered this pixel color
    texture_color: Option<RgbaPixel>,

    /// the index of the layer that this
    /// object exists on
    layer: usize,

    current_bounds: Rect,
    previous_bounds: Option<Rect>,
}

#[derive(Debug, Default)]
pub struct AboveRegions {
    above_my_current: Vec<Rect>,
    above_my_previous: Vec<Rect>,
}

#[derive(Debug)]
pub struct BelowRegion {
    region: Rect,
    /// the object index
    region_belongs_to: usize,
}

#[derive(Debug, Default)]
pub struct BelowRegions {
    below_my_previous: Vec<BelowRegion>,
}

pub enum ObjectTextureType {
    ObjectTextureColor(RgbaPixel),
    ObjectTextureVec(Vec<u8>),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct RgbaPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Copy, Clone)]
pub enum DrawPixels<'a> {
    PixelVec(&'a Vec<u8>),
    PixelColor(RgbaPixel),
}

#[derive(Copy, Clone, Debug, PartialEq)]
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

pub fn should_skip_point(skip_regions: &Vec<Rect>, x: u32, y: u32) -> bool {
    for rect in skip_regions {
        if rect.contains(x, y) { return true };
    }
    false
}

impl ManagedLayer {
    /// ManagedLayer.objects[] contains a vec of object indices
    /// that exist on the PortionRenderer. this method takes one of those
    /// indices and returns an option of the local index of ManagedLayer.objects
    /// where that index exists
    pub fn get_local_object_index(&self, object_index: usize) -> Option<usize> {
        self.objects.iter().position(|p| *p == object_index)
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
        p.pix_h = height;
        p.pix_w = width;
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

    /// returns (num_rows, num_cols)
    pub fn get_grid_dimensions(&self) -> (usize, usize) {
        let num_rows = self.grid.rows();
        let num_cols = self.grid.cols();
        (num_rows, num_cols)
    }

    /// iterates over the grid, and returns the minimum
    /// amount of contiguous active portions, and then
    /// resets the grid to not active
    pub fn flush_portions(&mut self) -> Vec<Rect> {
        let num_rows = self.grid.rows();
        let num_cols = self.grid.cols();

        // debug mode:
        if cfg!(test) {
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

/// Dont use this in a real program
/// this is just convenient for debugging
/// the issue with using this in a real program is its not very
/// performant if you are accessing many pixels at once
/// because the indices (x, y) are probably not cached
/// between calls to the .index((x, y)) method
impl Index<(u32, u32)> for PortionRenderer {
    type Output = [u8];

    fn index(&self, index: (u32, u32)) -> &Self::Output {
        let red_index = get_red_index!(index.0, index.1, self.width, self.indices_per_pixel) as usize;
        self.pixel_buffer.get(red_index..(red_index+4)).expect("Pixel out of bounds")
    }
}

impl From<&[u8]> for RgbaPixel {
    fn from(orig: &[u8]) -> Self {
        if orig.len() < 4 {
            panic!("Cannot convert {:#?} to RgbaPixel", orig);
        }
        RgbaPixel {
            r: orig[0],
            g: orig[1],
            b: orig[2],
            a: orig[3],
        }
    }
}

impl AsRef<Portioner> for PortionRenderer {
    fn as_ref(&self) -> &Portioner { &self.portioner }
}
impl AsMut<Portioner> for PortionRenderer {
    fn as_mut(&mut self) -> &mut Portioner { &mut self.portioner }
}
impl AsMut<Vec<u8>> for PortionRenderer {
    fn as_mut(&mut self) -> &mut Vec<u8> { &mut self.pixel_buffer }
}

impl PortionRenderer {
    pub fn new(
        width: u32,
        height: u32,
        num_rows: u32,
        num_cols: u32,
    ) -> PortionRenderer {
        let indices_per_pixel = 4; // TODO: dont assume
        let num_pixels = width * height;
        let data_len: usize = (num_pixels * indices_per_pixel) as usize;
        let pixel_buffer = vec![0; data_len];
        PortionRenderer {
            clear_buffer: pixel_buffer.clone(),
            pixel_buffer,
            width,
            height,
            indices_per_pixel,
            layers: vec![ManagedLayer { index: 0, objects: vec![], updates: vec![], }],
            textures: vec![],
            objects: vec![],
            portioner: Portioner::new(width, height, num_rows, num_cols),
        }
    }

    /// clones the current visible buffer to the clear buffer
    /// useful when you want to render an intial scene, and
    /// then use that as the background
    pub fn set_clear_buffer(&mut self) {
        self.clear_buffer = self.pixel_buffer.clone();
    }

    /// returns the layer's actual index of the Vec its in,
    /// whereas the layer_index: u32 is a human friendly index
    /// like 0, 1000, 1001, etc.
    pub fn get_or_make_layer(&mut self, layer_index: u32) -> usize {
        let mut insert_at_index = 0;
        let mut update_at_index = None;
        let last_i = self.layers.len() - 1;
        for (i, layer) in self.layers.iter().enumerate() {
            if layer.index == layer_index {
                update_at_index = Some(i);
                break;
            } else if layer.index > layer_index {
                insert_at_index = i;
                break;
            } else if i == last_i {
                insert_at_index = i + 1;
                break;
            }
        }

        if let Some(i) = update_at_index {
            i
        } else {
            self.layers.push(ManagedLayer {
                index: layer_index,
                objects: vec![],
                updates: vec![],
            });
            insert_at_index
        }
    }

    pub fn create_object(&mut self, layer_index: u32, bounds: Rect, texture: ObjectTextureType) -> usize {
        let (texture_index, texture_color) = match texture {
            ObjectTextureType::ObjectTextureColor(c) => {
                (0, Some(c))
            }
            ObjectTextureType::ObjectTextureVec(v) => {
                let next_index = self.textures.len();
                self.textures.push(v);
                (next_index, None)
            }
        };
        let new_object_index = self.objects.len();
        let layer_index = self.get_or_make_layer(layer_index);
        let new_object = Object {
            texture_index: texture_index,
            texture_color: texture_color,
            current_bounds: bounds,
            previous_bounds: None,
            layer: layer_index,
        };
        self.objects.push(new_object);
        let updated_object_index = self.layers[layer_index].objects.len();
        self.layers[layer_index].objects.push(new_object_index);
        self.layers[layer_index].updates.push(updated_object_index);
        new_object_index
    }

    pub fn create_object_from_color(&mut self, layer_index: u32, bounds: Rect, color: RgbaPixel) -> usize {
        self.create_object(layer_index, bounds, ObjectTextureType::ObjectTextureColor(color))
    }

    pub fn create_object_from_texture(&mut self, layer_index: u32, bounds: Rect, texture: Vec<u8>) -> usize {
        self.create_object(layer_index, bounds, ObjectTextureType::ObjectTextureVec(texture))
    }

    pub fn draw_grid_outline(&mut self) {
        draw_grid_outline(&self.portioner, &mut self.pixel_buffer, self.indices_per_pixel);
    }

    pub fn object_needs_drawing(&mut self, object_index: usize) -> bool {
        let object = &self.objects[object_index];
        match object.previous_bounds {
            Some(prev_bounds) => {
                let current_bounds = object.current_bounds;
                current_bounds.x != prev_bounds.x ||
                current_bounds.y != prev_bounds.y ||
                current_bounds.w != prev_bounds.w ||
                current_bounds.h != prev_bounds.h
            }
            None => true,
        }
    }

    /// layer_index is usize of the index of the layer as in PortionRenderer.layers[layer_index]
    /// this method returns an object containing rect regions that are above this current object
    /// so these regions should then be ignored when drawing this object, both for clearing
    /// its previous pixels, or updating its new pixels
    pub fn get_regions_above_object(&self, object_index: usize, layer_index: usize) -> AboveRegions {
        // layer_index is the index of the layer that this
        // object is on, so we check the layers above it:
        let start_layer_check_at = layer_index + 1;
        let layers = self.layers.len();
        let object_current_bounds = &self.objects[object_index].current_bounds;
        let object_previous_bounds = &self.objects[object_index].previous_bounds;
        let mut above_bounds = AboveRegions::default();
        for i in start_layer_check_at..layers {
            let layer = &self.layers[i];
            for layer_object_index in layer.objects.iter() {
                let layer_object = &self.objects[*layer_object_index];
                if let Some(intersection) = Rect::intersection(layer_object.current_bounds, *object_current_bounds) {
                    above_bounds.above_my_current.push(intersection);
                }
                if let Some(object_previous) = object_previous_bounds {
                    if let Some(intersection) = Rect::intersection(layer_object.current_bounds, *object_previous) {
                        above_bounds.above_my_previous.push(intersection);
                    }
                }
            }
        }
        above_bounds
    }

    /// similar to get_regions_above_object, except we iterate the layers in reverse
    /// and find the regions underneath us that were previously covered up, but are now
    /// open, so they should be drawn again
    pub fn get_regions_below_object(&self, object_index: usize, layer_index: usize) -> BelowRegions {
        // no need to check anything if we are at the bottom layer
        if layer_index == 0 {
            return BelowRegions::default();
        }
        // if theres no previous bounds, we dont need to iterate
        let object_previous_bounds = match &self.objects[object_index].previous_bounds {
            None => return BelowRegions::default(),
            Some(region) => region,
        };
        let mut below_bounds = BelowRegions::default();
        let start_layer_check_at = layer_index;
        for i in (0..start_layer_check_at).rev() {
            let layer = &self.layers[i];
            for layer_object_index in layer.objects.iter() {
                let layer_object = &self.objects[*layer_object_index];
                if let Some(intersection) = Rect::intersection(layer_object.current_bounds, *object_previous_bounds) {
                    below_bounds.below_my_previous.push(BelowRegion {
                        region: intersection,
                        region_belongs_to: *layer_object_index,
                    });
                }
            }
        }

        below_bounds
    }

    pub fn draw_all_layers(&mut self) {
        // TODO: can we avoid drawing bottom layers
        // if a top layer fully covers it up?
        let mut draw_object_indices = vec![];
        for (layer_index, layer) in self.layers.iter_mut().enumerate() {
            // make sure to drain so we remove these updates
            // and prevent them from showing up next draw
            for update_index in layer.updates.drain(..) {
                let object_index = layer.objects[update_index];
                draw_object_indices.push((layer_index, object_index));
            }
        }

        for (layer_index, object_index) in draw_object_indices {
            let above_regions = self.get_regions_above_object(object_index, layer_index);
            let below_regions = self.get_regions_below_object(object_index, layer_index);
            self.draw_object(object_index, above_regions, below_regions);
        }
    }

    pub fn set_layer_update(&mut self, object_index: usize) {
        let layer_index = self.objects[object_index].layer;
        if let Some(local_object_index) = self.layers[layer_index].get_local_object_index(object_index) {
            self.layers[layer_index].updates.push(local_object_index);
        }
    }

    pub fn move_object_x_by(&mut self, object_index: usize, by: i32) {
        if by < 0 {
            let current_x = self.objects[object_index].current_bounds.x;
            let by = (0 - by) as u32;
            if current_x >= by {
                self.objects[object_index].current_bounds.x -= by;
                self.set_layer_update(object_index);
            }
        } else {
            self.objects[object_index].current_bounds.x += by as u32;
            self.set_layer_update(object_index);
        }
    }

    pub fn move_object_y_by(&mut self, object_index: usize, by: i32) {
        if by < 0 {
            let current_y = self.objects[object_index].current_bounds.y;
            let by = (0 - by) as u32;
            if current_y >= by {
                self.objects[object_index].current_bounds.y -= by;
                self.set_layer_update(object_index);
            }
        } else {
            self.objects[object_index].current_bounds.y += by as u32;
            self.set_layer_update(object_index);
        }
    }

    pub fn get_pixel_from_object_at(&self, object_index: usize, x: u32, y: u32) -> Option<RgbaPixel> {
        if let Some(pixel) = self.objects[object_index].texture_color {
            return Some(pixel);
        }

        let texture_index = self.objects[object_index].texture_index;
        let current_bounds = self.objects[object_index].current_bounds;
        // it should be guaranteed that x and y exist within the objects current bounds
        if x < current_bounds.x || y < current_bounds.y {
            panic!("Called get_pixel_from_object_at with ({}, {}) but objects bounds are {:?}", x, y, current_bounds);
        }
        let local_x = x - current_bounds.x;
        let local_y = y - current_bounds.y;
        let red_index = get_red_index!(local_x, local_y, current_bounds.w, self.indices_per_pixel) as usize;
        let pixel: RgbaPixel = match self.textures[texture_index].get(red_index..(red_index+4)) {
            Some(u8_slice) => u8_slice.into(),
            None => return None,
        };
        Some(pixel)
    }

    pub fn clear_pixels_from_below_object(&mut self, pb_red_index: usize, x: u32, y: u32, skip_below: &BelowRegions) -> bool {
        for below in skip_below.below_my_previous.iter() {
            if below.region.contains(x, y) {
                let pixel = self.get_pixel_from_object_at(
                    below.region_belongs_to, x, y
                );
                if let Some(pixel) = pixel {
                    // println!("Undoing ({}, {}) via clearBelow: {:?}", x, y, pixel);
                    self.pixel_buffer[pb_red_index] = pixel.r;
                    self.pixel_buffer[pb_red_index + 1] = pixel.g;
                    self.pixel_buffer[pb_red_index + 2] = pixel.b;
                    self.pixel_buffer[pb_red_index + 3] = pixel.a;
                    return true;
                } else {
                    return false;
                }
            }
        }
        false
    }

    pub fn draw(&mut self, pixels: &[u8], bounds: Rect) {
        let x = bounds.x;
        let y = bounds.y;
        let w = bounds.w;
        let h = bounds.h;
        let mut pixels_index = 0;
        for i in y..(y + h) {
            for j in x..(x + w) {
                let red_index = get_red_index!(j, i, self.width, self.indices_per_pixel) as usize;
                self.pixel_buffer[red_index] = pixels[pixels_index];
                self.pixel_buffer[red_index + 1] = pixels[pixels_index + 1];
                self.pixel_buffer[red_index + 2] = pixels[pixels_index + 2];
                self.pixel_buffer[red_index + 3] = pixels[pixels_index + 3];

                pixels_index += 4;
            }
        }
    }

    pub fn draw_object(&mut self, object_index: usize, skip_above: AboveRegions, skip_below: BelowRegions) {
        // println!("\n----------------");
        let previous_bounds = self.objects[object_index].previous_bounds;
        if let Some(prev) = previous_bounds {
            // println!("Undoing region: {:#?}", prev);
            // println!("Skip below is: {:#?}", skip_below);
            let should_try_clear_below = !skip_below.below_my_previous.is_empty();
            let prev_x = prev.x;
            let prev_y = prev.y;
            let prev_w = prev.w;
            let prev_h = prev.h;
            for i in prev_y..(prev_y + prev_h) {
                for j in prev_x..(prev_x + prev_w) {
                    if should_skip_point(&skip_above.above_my_previous, j, i) {
                        // println!("Skipping undo of: ({}, {})", j, i);
                        continue;
                    }
                    self.portioner.take_pixel(j, i);
                    let red_index = get_red_index!(j, i, self.width, self.indices_per_pixel);
                    let red_index = red_index as usize;

                    // try to clear this pixel from what was
                    // underneath it first
                    if should_try_clear_below && self.clear_pixels_from_below_object(
                        red_index, j, i, &skip_below
                    ) { continue; }

                    // if that fails, use the clear buffer
                    // let clearpix = RgbaPixel {
                    //     r: self.clear_buffer[red_index],
                    //     g: self.clear_buffer[red_index + 1],
                    //     b: self.clear_buffer[red_index + 2],
                    //     a: self.clear_buffer[red_index + 3],
                    // };
                    // println!("Undoing ({}, {}) via clearbuffer: {:?}", j, i, clearpix);
                    self.pixel_buffer[red_index] = self.clear_buffer[red_index];
                    self.pixel_buffer[red_index + 1] = self.clear_buffer[red_index + 1];
                    self.pixel_buffer[red_index + 2] = self.clear_buffer[red_index + 2];
                    self.pixel_buffer[red_index + 3] = self.clear_buffer[red_index + 3];
                }
            }
        }

        let mut object = &mut self.objects[object_index];
        let now = object.current_bounds;
        // println!("Going to draw everything within: {:#?}", now);
        let now_x = now.x;
        let now_y = now.y;
        let now_w = now.w;
        let now_h = now.h;
        // println!("Except: {:#?}", skip_regions);
        let item_pixels = match object.texture_color {
            Some(rgba_pixel) => {
                for i in now_y..(now_y + now_h) {
                    for j in now_x..(now_x + now_w) {
                        if should_skip_point(&skip_above.above_my_current, j, i) {
                            // println!("Skipping: ({}, {})", j, i);
                            continue;
                        }
                        self.portioner.take_pixel(j, i);
                        let red_index = get_red_index!(j, i, self.width, self.indices_per_pixel);
                        let red_index = red_index as usize;
                        // TODO: pixel format???
                        // println!("Overwriting ({}, {}) with {:?}", j, i, rgba_pixel);
                        self.pixel_buffer[red_index] = rgba_pixel.r;
                        self.pixel_buffer[red_index + 1] = rgba_pixel.g;
                        self.pixel_buffer[red_index + 2] = rgba_pixel.b;
                        self.pixel_buffer[red_index + 3] = rgba_pixel.a;
                    }
                }
                object.previous_bounds = Some(object.current_bounds);
                return;
            }
            None => {
                &self.textures[object.texture_index]
            }
        };

        // if we got here then that means item.get_current_pixels
        // returns an actual vec of pixels, so iterate over those
        // and keep track of the pixel index... its up to
        // the item to ensure that this vec of pixels is the same
        // dimension as the bounds it gave us in item.get_current_bounds()...
        let mut item_pixel_index = 0;
        for i in now_y..(now_y + now_h) {
            for j in now_x..(now_x + now_w) {
                if should_skip_point(&skip_above.above_my_current, j, i) {
                    // println!("Skipping: ({}, {})", j, i);
                    continue;
                }
                self.portioner.take_pixel(j, i);
                let red_index = get_red_index!(j, i, self.width, self.indices_per_pixel);
                let red_index = red_index as usize;
                // TODO: pixel format???
                self.pixel_buffer[red_index] = item_pixels[item_pixel_index];
                self.pixel_buffer[red_index + 1] = item_pixels[item_pixel_index + 1];
                self.pixel_buffer[red_index + 2] = item_pixels[item_pixel_index + 2];
                self.pixel_buffer[red_index + 3] = item_pixels[item_pixel_index + 3];
                item_pixel_index += 4;
            }
        }
        object.previous_bounds = Some(object.current_bounds);
    }
}

pub fn draw_grid_outline(
    p: &Portioner,
    pixel_buffer: &mut Vec<u8>,
    indices_per_pixel: u32,
) {
    let width = p.pix_w;
    let height = p.pix_h;
    let row_height = p.row_height;
    let col_width = p.col_width;
    let mut i = 0;
    while i < height {
        for j in 0..width {
            // (j, i) is the pixel index
            // but the pixel buffer has 4 values per pixel: RGBA
            let red_index = get_red_index!(j, i, width, indices_per_pixel);
            let index = red_index as usize;
            pixel_buffer[index] = 100;
            pixel_buffer[index + 1] = 100;
            pixel_buffer[index + 2] = 100;
            pixel_buffer[index + 3] = 100;
        }

        i += row_height;
    }

    // now i will be x, and j will be y
    let mut i = 0;
    while i < width {
        for j in 0..height {
            let red_index = get_red_index!(i, j, width, indices_per_pixel);
            let index = red_index as usize;
            pixel_buffer[index] = 100;
            pixel_buffer[index + 1] = 100;
            pixel_buffer[index + 2] = 100;
            pixel_buffer[index + 3] = 100;
        }

        i += col_width;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_WIDTH: u32 = 800;
    const TEST_HEIGHT: u32 = 600;
    const PIX1: RgbaPixel = RgbaPixel { r: 1, g: 1, b: 1, a: 1 };
    const PIX2: RgbaPixel = RgbaPixel { r: 2, g: 2, b: 2, a: 2 };
    const PIX3: RgbaPixel = RgbaPixel { r: 3, g: 3, b: 3, a: 3 };
    const PIX4: RgbaPixel = RgbaPixel { r: 4, g: 4, b: 4, a: 4 };

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

    #[test]
    fn managed_layering_works() {
        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );
        assert_eq!(p.layers.len(), 1);
        p.create_object_from_color(0,
            Rect { x: 0, y: 0, w: 0, h: 0 },
            PIXEL_BLACK,
        );
        assert_eq!(p.layers.len(), 1);
        assert_eq!(p.objects.len(), 1);
        assert_eq!(p.textures.len(), 0);
        assert_eq!(p.layers[0].objects.len(), 1);
    }

    fn assert_pixels_in_map(p: &mut PortionRenderer, map: &[char], width: u32) {
        let mut x = 0;
        let mut expected_string = String::from("[");
        for pixel_color in map {
            expected_string.push(*pixel_color);
            expected_string.push_str(", ");
            x += 1;
            if x >= width {
                expected_string.push_str("\n ");
                x = 0;
            }
        }

        let mut x = 0;
        let mut y = 0;
        let mut actual_string = String::from("[");
        let mut should_panic = false;
        for pixel_color in map {
            let pixel_slice: RgbaPixel = p[(x, y)].into();
            let mut should_newline = false;

            x += 1;
            if x >= width {
                x = 0;
                y += 1;
                should_newline = true;
            }

            let pixel_compare = match pixel_color {
                'g' => PIXEL_GREEN,
                'r' => PIXEL_RED,
                'b' => PIXEL_BLUE,
                'x' => PIXEL_BLACK,
                '1' => PIX1,
                '2' => PIX2,
                '3' => PIX3,
                '4' => PIX4,
                c => panic!("Found undefined char in map: {}", c),
            };
            let c = match pixel_slice {
                PIXEL_BLACK => 'x',
                PIXEL_GREEN => 'g',
                PIXEL_RED => 'r',
                PIXEL_BLUE => 'b',
                PIX1 => '1',
                PIX2 => '2',
                PIX3 => '3',
                PIX4 => '4',
                _ => '?',
            };
            actual_string.push(c);
            actual_string.push_str(", ");
            if should_newline {
                actual_string.push_str("\n ");
            }
            if pixel_compare != pixel_slice {
                // panic!("\n\nExpected {:?}\nFound {:?}\n at index ({}, {})\n\n", pixel_compare, pixel_slice, debug_x, debug_y);
                should_panic = true;
            }
        }
        if should_panic {
            panic!("\n\nExpected\n{}\nActual\n{}\n", expected_string, actual_string);
        }
    }

    fn texture_from(pixels: &[RgbaPixel]) -> Vec<u8> {
        let mut out_vec = vec![];
        for p in pixels {
            out_vec.push(p.r);
            out_vec.push(p.g);
            out_vec.push(p.b);
            out_vec.push(p.a);
        }
        out_vec
    }

    #[test]
    fn simple_texture_move_works() {
        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );
        let textured = p.create_object_from_texture(
            0, Rect { x: 0, y: 0, w: 2, h: 2 },
            texture_from(&[PIX1, PIX2, PIX3, PIX4]),
        );
        p.draw_all_layers();
        let assert_map = [
            '1', '2', 'x', 'x',
            '3', '4', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);

        p.move_object_x_by(textured, 1);
        p.draw_all_layers();
        let assert_map = [
            'x', '1', '2', 'x',
            'x', '3', '4', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);
    }

    #[test]
    fn getting_pixel_from_object_at_position_works() {
        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );
        let textured = p.create_object_from_texture(
            0, Rect { x: 2, y: 1, w: 2, h: 2 },
            texture_from(&[PIX1, PIX2, PIX3, PIX4]),
        );
        p.draw_all_layers();
        let assert_map = [
            'x', 'x', 'x', 'x',
            'x', 'x', '1', '2',
            'x', 'x', '3', '4',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);

        let pixel = p.get_pixel_from_object_at(textured, 2, 1).unwrap();
        assert_eq!(pixel, PIX1);
        let pixel = p.get_pixel_from_object_at(textured, 3, 1).unwrap();
        assert_eq!(pixel, PIX2);
        let pixel = p.get_pixel_from_object_at(textured, 2, 2).unwrap();
        assert_eq!(pixel, PIX3);
        let pixel = p.get_pixel_from_object_at(textured, 3, 2).unwrap();
        assert_eq!(pixel, PIX4);
    }

    #[test]
    fn simple_overlap_works() {
        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );
        let _green = p.create_object_from_color(
            0, Rect { x: 0, y: 0, w: 2, h: 2 },
            PIXEL_GREEN
        );
        let red = p.create_object_from_color(
            1, Rect { x: 2, y: 0, w: 2, h: 2 },
            PIXEL_RED
        );
        p.draw_all_layers();

        // top left box should be all green, next to
        // it should be all red
        let assert_map = [
            'g', 'g', 'r', 'r',
            'g', 'g', 'r', 'r',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);

        // now if red moves left one pixel
        // then it should cover up half of the green
        // box because red is 1 layer higher than green
        // and one col to the right of the red box
        // should now be black because red doesnt exist there anymore
        p.move_object_x_by(red, -1);
        p.draw_all_layers();

        let assert_map = [
            'g', 'r', 'r', 'x',
            'g', 'r', 'r', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);
    }

    #[test]
    fn simple_underlap_works() {
        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );
        let green = p.create_object_from_color(
            0, Rect { x: 0, y: 0, w: 2, h: 2 },
            PIXEL_GREEN
        );
        let _red = p.create_object_from_color(
            1, Rect { x: 2, y: 0, w: 2, h: 2 },
            PIXEL_RED
        );
        p.draw_all_layers();

        // top left box should be all green, next to
        // it should be all red
        let assert_map = [
            'g', 'g', 'r', 'r',
            'g', 'g', 'r', 'r',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);

        // now if green moves right one pixel
        // then it should be under half of red
        // box because red is 1 layer higher than green
        // and one col to the left of the green box
        // should now be black because green doesnt exist there anymore
        p.move_object_x_by(green, 1);
        p.draw_all_layers();

        let assert_map = [
            'x', 'g', 'r', 'r',
            'x', 'g', 'r', 'r',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);
    }

    #[test]
    fn simple_overlap_move_works() {
        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );
        let green = p.create_object_from_color(
            0, Rect { x: 0, y: 0, w: 2, h: 2 },
            PIXEL_GREEN
        );
        let red = p.create_object_from_color(
            1, Rect { x: 2, y: 0, w: 2, h: 2 },
            PIXEL_RED
        );
        println!("ONE");
        p.draw_all_layers();

        // top left box should be all green, next to
        // it should be all red
        let assert_map = [
            'g', 'g', 'r', 'r',
            'g', 'g', 'r', 'r',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);

        p.move_object_x_by(red, -1);
        println!("TWO");
        p.draw_all_layers();

        let assert_map = [
            'g', 'r', 'r', 'x',
            'g', 'r', 'r', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);

        // now we test if red moves out of the way, that
        // green will be shown, and the rest of the pixels are black
        p.move_object_x_by(red, 3);
        println!("THREE");
        p.draw_all_layers();
        let assert_map = [
            'g', 'g', 'x', 'x',
            'g', 'g', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);

        // now if green moves down and out of the way, then the places under
        // green should be black
        p.move_object_y_by(green, 3);
        println!("FOUR");
        p.draw_all_layers();
        let assert_map = [
            'x', 'x', 'x', 'x',
            'x', 'x', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);
    }

    #[test]
    fn simple_underlap_move_works() {
        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );
        let green = p.create_object_from_color(
            0, Rect { x: 0, y: 0, w: 2, h: 2 },
            PIXEL_GREEN
        );
        let red = p.create_object_from_color(
            1, Rect { x: 2, y: 0, w: 2, h: 2 },
            PIXEL_RED
        );
        println!("One");
        p.draw_all_layers();

        // top left box should be all green, next to
        // it should be all red
        let assert_map = [
            'g', 'g', 'r', 'r',
            'g', 'g', 'r', 'r',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);

        println!("Two:");
        p.move_object_x_by(green, 1);
        p.draw_all_layers();

        let assert_map = [
            'x', 'g', 'r', 'r',
            'x', 'g', 'r', 'r',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);

        println!("Three!:");

        // now if red moves down one, then the portion
        // of green that was previously under red
        // should be visible
        p.move_object_y_by(red, 1);
        p.draw_all_layers();

        let assert_map = [
            'x', 'g', 'g', 'x',
            'x', 'g', 'r', 'r',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);
    }

    #[test]
    fn simple_underlap_move_gets_proper_above_and_below_bounds() {
        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );
        let green = p.create_object_from_color(
            0, Rect { x: 0, y: 0, w: 2, h: 2 },
            PIXEL_GREEN
        );
        let red = p.create_object_from_color(
            1, Rect { x: 2, y: 0, w: 2, h: 2 },
            PIXEL_RED
        );
        println!("One");
        p.draw_all_layers();

        // top left box should be all green, next to
        // it should be all red
        let assert_map = [
            'g', 'g', 'r', 'r',
            'g', 'g', 'r', 'r',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);

        println!("Two:");
        p.move_object_x_by(green, 1);
        // should look like:
        // let assert_map = [
        //     'x', 'g', 'r', 'r',
        //     'x', 'g', 'r', 'r',
        // ];
        let above_bounds = p.get_regions_above_object(green, 0);
        assert_eq!(above_bounds.above_my_previous.len(), 0);
        assert_eq!(above_bounds.above_my_current.len(), 1);
        assert_eq!(
            above_bounds.above_my_current[0],
            Rect { x: 2, y: 0, w: 1, h: 2 },
        );
        p.draw_all_layers();

        // // now if red moves down one, then the portion
        // // of green that was previously under red
        // // should be visible
        p.move_object_y_by(red, 1);
        // should look like:
        // let assert_map = [
        //     'x', 'g', 'g', 'x',
        //     'x', 'g', 'r', 'r',
        // ];
        let below_bounds = p.get_regions_below_object(red, 1);
        assert_eq!(below_bounds.below_my_previous.len(), 1);
        assert_eq!(
            below_bounds.below_my_previous[0].region,
            Rect { x: 2, y: 0, w: 1, h: 2 },
            // technically height should be 1, but the bottom pixel
            // that is currently red will just be overridden anyway
            // so this is acceptable for now
        )
    }

    #[test]
    fn simple_underlap_move_simulatenous_works() {
        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );
        let green = p.create_object_from_color(
            0, Rect { x: 0, y: 0, w: 2, h: 2 },
            PIXEL_GREEN
        );
        let red = p.create_object_from_color(
            1, Rect { x: 2, y: 0, w: 2, h: 2 },
            PIXEL_RED
        );
        p.draw_all_layers();

        // top left box should be all green, next to
        // it should be all red
        let assert_map = [
            'g', 'g', 'r', 'r',
            'g', 'g', 'r', 'r',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);

        p.move_object_x_by(green, 1);
        p.draw_all_layers();

        let assert_map = [
            'x', 'g', 'r', 'r',
            'x', 'g', 'r', 'r',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);

        println!("this one!:");

        // now if green and red move out of the way at the same time,
        // then the overlapping portion should go back to black
        p.move_object_x_by(red, 2);
        p.move_object_y_by(green, 2);
        p.draw_all_layers();

        let assert_map = [
            'x', 'x', 'x', 'x',
            'x', 'x', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);
    }

    #[test]
    fn simple_underlap_move_sequential_works() {
        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );
        let green = p.create_object_from_color(
            0, Rect { x: 0, y: 0, w: 2, h: 2 },
            PIXEL_GREEN
        );
        let red = p.create_object_from_color(
            1, Rect { x: 2, y: 0, w: 2, h: 2 },
            PIXEL_RED
        );
        p.draw_all_layers();

        // top left box should be all green, next to
        // it should be all red
        let assert_map = [
            'g', 'g', 'r', 'r',
            'g', 'g', 'r', 'r',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);

        p.move_object_x_by(green, 1);
        p.draw_all_layers();

        let assert_map = [
            'x', 'g', 'r', 'r',
            'x', 'g', 'r', 'r',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);

        println!("ONE:");
        // now if green moves out of the way, then render
        // then red moves out of the way, then render, then
        // the overlapping portion should now be black
        p.move_object_y_by(green, 2);
        p.draw_all_layers();
        println!("TWO:");
        p.move_object_x_by(red, 2);
        p.draw_all_layers();

        let assert_map = [
            'x', 'x', 'x', 'x',
            'x', 'x', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);
    }
}
