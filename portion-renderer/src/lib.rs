use std::cmp;
use std::ops::Index;

pub mod portioner;
pub mod projection;
pub mod transform;
pub use projection::Projection;
pub use transform::*;
pub use portioner::*;

#[macro_export]
macro_rules! get_red_index {
    ($x:expr, $y:expr, $w:expr, $indices_per_pixel:expr) => {
        $y * ($w * $indices_per_pixel) + ($x * $indices_per_pixel)
    };
}

pub const PIXEL_BLANK: RgbaPixel = RgbaPixel { r: 0, g: 0, b: 0, a: 0 };
pub const PIXEL_BLACK: RgbaPixel = RgbaPixel { r: 0, g: 0, b: 0, a: 255 };
pub const PIXEL_RED: RgbaPixel = RgbaPixel { r: 255, g: 0, b: 0, a: 255 };
pub const PIXEL_GREEN: RgbaPixel = RgbaPixel { r: 0, g: 255, b: 0, a: 255 };
pub const PIXEL_BLUE: RgbaPixel = RgbaPixel { r: 0, g: 0, b: 255, a: 255 };

pub struct PortionRenderer {
    pixel_buffer: Vec<u8>,
    clear_buffer: Vec<u8>,
    portioner: Portioner,

    width: u32,
    height: u32,
    indices_per_pixel: u32, // probably only 3 or 4

    textures: Vec<Texture>,
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

#[derive(Copy, Clone)]
pub enum ObjectRenderMode {
    /// RenderAsMuchAsPossible will
    /// render all of the objects texture that exists
    /// and then stops if the texture is smaller than the objects bounds.
    /// if the texture is larger than the bounds, then this mode
    /// will render up to the bounds, and then discard the rest of
    /// the texture
    RenderAsMuchAsPossible,
    /// RenderToFit will always stretch the texture
    /// to fit it into the bounds
    RenderToFit,
}

pub struct Texture {
    data: Vec<u8>,
    width: usize,
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

    render_mode: ObjectRenderMode,

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
    ObjectTextureVec(Vec<u8>, usize),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct RgbaPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

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

pub fn pixel_vec_to_texture(pixel_vec: Vec<RgbaPixel>) -> Vec<u8> {
    let mut out_vec = vec![];

    for pixel in pixel_vec {
        out_vec.push(pixel.r);
        out_vec.push(pixel.g);
        out_vec.push(pixel.b);
        out_vec.push(pixel.a);
    }

    out_vec
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
            ObjectTextureType::ObjectTextureVec(v, w) => {
                let next_index = self.textures.len();
                self.textures.push(Texture {
                    data: v,
                    width: w,
                });
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
            render_mode: ObjectRenderMode::RenderAsMuchAsPossible,
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

    pub fn create_object_from_texture(&mut self, layer_index: u32, bounds: Rect, texture: Vec<u8>, texture_width: usize) -> usize {
        self.create_object(layer_index, bounds, ObjectTextureType::ObjectTextureVec(texture, texture_width))
    }

    pub fn create_object_from_texture_exact(&mut self, layer_index: u32, bounds: Rect, texture: Vec<u8>) -> usize {
        self.create_object_from_texture(layer_index, bounds, texture, bounds.w as usize)
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

    pub fn set_object_render_mode(&mut self, object_index: usize, render_mode: ObjectRenderMode) {
        self.objects[object_index].render_mode = render_mode;
    }

    pub fn get_pixel_from_object_at(&self, object_index: usize, x: u32, y: u32) -> Option<RgbaPixel> {
        if let Some(pixel) = self.objects[object_index].texture_color {
            return Some(pixel);
        }

        let texture_index = self.objects[object_index].texture_index;
        let current_bounds = self.objects[object_index].current_bounds;
        let render_mode = self.objects[object_index].render_mode;
        let texture_width = self.textures[texture_index].width;
        // it should be guaranteed that x and y exist within the objects current bounds
        if x < current_bounds.x || y < current_bounds.y {
            panic!("Called get_pixel_from_object_at with ({}, {}) but objects bounds are {:?}", x, y, current_bounds);
        }

        let local_x = x - current_bounds.x;
        let local_y = y - current_bounds.y;
        if let ObjectRenderMode::RenderAsMuchAsPossible = render_mode {
            let red_index = get_red_index!(local_x, local_y, current_bounds.w, self.indices_per_pixel) as usize;
            let pixel: RgbaPixel = match self.textures[texture_index].data.get(red_index..(red_index+4)) {
                Some(u8_slice) => u8_slice.into(),
                None => return None,
            };
            Some(pixel)
        } else if let ObjectRenderMode::RenderToFit = render_mode {
            let texture_pixels_len = self.textures[texture_index].data.len();
            let indices_per_pixel = self.indices_per_pixel as usize;
            let texture_height = (texture_pixels_len / indices_per_pixel) / texture_width;
            let width_stretch_factor = current_bounds.w / texture_width as u32;
            let height_stretch_factor = current_bounds.h / texture_height as u32;

            let local_x = local_x / width_stretch_factor;
            let local_y = local_y / height_stretch_factor;
            let local_x = if local_x >= texture_width as u32 {
                texture_width as u32 - 1
            } else { local_x };
            let local_y = if local_y >= texture_height as u32 {
                texture_height as u32 - 1
            } else { local_y };

            let red_index = get_red_index!(local_x, local_y, texture_width as u32, self.indices_per_pixel) as usize;
            let pixel: RgbaPixel = match self.textures[texture_index].data.get(red_index..(red_index+indices_per_pixel)) {
                Some(u8_slice) => u8_slice.into(),
                None => return None,
            };
            Some(pixel)
        } else {
            None
        }
    }

    pub fn clear_pixels_from_below_object(&mut self, pb_red_index: usize, x: u32, y: u32, skip_below: &BelowRegions) -> bool {
        for below in skip_below.below_my_previous.iter() {
            if below.region.contains(x, y) {
                let pixel = self.get_pixel_from_object_at(
                    below.region_belongs_to, x, y
                );
                if let Some(pixel) = pixel {
                    if pixel.a == 0 {
                        return false;
                    }
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

    pub fn draw_stretched(&mut self, pixels: &[u8], pixel_width: u32, pixel_height: u32) {
        let width_stretch_factor = self.width / pixel_width;
        let height_stretch_factor = self.height / pixel_height;
        let width = self.width;
        let height = self.height;
        let mut pixel_y = 0;
        for i in 0..height {
            let mut pixel_x = 0;
            for j in 0..width {
                let red_index = get_red_index!(j, i, self.width, self.indices_per_pixel) as usize;
                let pixel_red_index = get_red_index!(pixel_x, pixel_y, pixel_width, 4) as usize;

                self.pixel_buffer[red_index] = pixels[pixel_red_index];
                self.pixel_buffer[red_index + 1] = pixels[pixel_red_index + 1];
                self.pixel_buffer[red_index + 2] = pixels[pixel_red_index + 2];
                self.pixel_buffer[red_index + 3] = pixels[pixel_red_index + 3];

                if j != 0 && j % width_stretch_factor == 0 {
                    pixel_x += 1;
                }
            }
            if i != 0 && i % height_stretch_factor == 0 {
                pixel_y += 1;
            }
        }
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
        let indices_per_pixel = self.indices_per_pixel as usize;
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
        let (item_pixels, item_width) = match object.texture_color {
            Some(rgba_pixel) => {
                // can skip rendering if the alpha is 0, no point in iterating
                if rgba_pixel.a == 0 {
                    object.previous_bounds = Some(object.current_bounds);
                    return;
                }

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
                (&self.textures[object.texture_index].data, self.textures[object.texture_index].width)
            }
        };

        // if we got here then that means item.get_current_pixels
        // returns an actual vec of pixels, so iterate over those
        // and keep track of the pixel index...
        if let ObjectRenderMode::RenderAsMuchAsPossible = object.render_mode {
            let item_pixels_max = item_pixels.len();
            let mut item_pixel_index = 0;
            'outer: for i in now_y..(now_y + now_h) {
                for j in now_x..(now_x + now_w) {
                    // if the alpha value is 0, skip this pixel
                    if item_pixels[item_pixel_index + 3] == 0 {
                        continue;
                    }
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
                    item_pixel_index += indices_per_pixel;
                    if item_pixel_index >= item_pixels_max {
                        break 'outer;
                    }
                }
            }
        } else if let ObjectRenderMode::RenderToFit = object.render_mode {
            let item_pixels_len = item_pixels.len();
            let item_height = (item_pixels_len / indices_per_pixel) / item_width;
            // TODO: handle shrink image?
            if now_w < item_width as u32 || now_h < item_height as u32 {
                panic!("texture shrinking not implemented yet");
            }
            let width_stretch_factor = now_w / item_width as u32;
            let height_stretch_factor = now_h / item_height as u32;
            let mut pixel_y = 0;
            for i in now_y..(now_y + now_h) {
                let i_diff = i - now_y;
                if i_diff != 0 && i_diff % height_stretch_factor == 0 {
                    pixel_y += 1;
                }
                if pixel_y >= item_height {
                    pixel_y = item_height - 1;
                }

                let mut pixel_x = 0;
                for j in now_x..(now_x + now_w) {
                    let j_diff = j - now_x;
                    if j_diff != 0 && j_diff % width_stretch_factor == 0 {
                        pixel_x += 1;
                    }
                    if pixel_x >= item_width {
                        pixel_x = item_width - 1;
                    }

                    let pixel_red_index = get_red_index!(pixel_x, pixel_y, item_width, indices_per_pixel) as usize;
                    // if alpha is 0, no point in checking above points, just skip
                    if item_pixels[pixel_red_index + 3] == 0 {
                        continue;
                    }

                    if should_skip_point(&skip_above.above_my_current, j, i) {
                        // println!("Skipping: ({}, {})", j, i);
                        continue;
                    }

                    self.portioner.take_pixel(j, i);
                    let red_index = get_red_index!(j, i, self.width, self.indices_per_pixel) as usize;

                    self.pixel_buffer[red_index] = item_pixels[pixel_red_index];
                    self.pixel_buffer[red_index + 1] = item_pixels[pixel_red_index + 1];
                    self.pixel_buffer[red_index + 2] = item_pixels[pixel_red_index + 2];
                    self.pixel_buffer[red_index + 3] = item_pixels[pixel_red_index + 3];
                }
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

    const PIX1: RgbaPixel = RgbaPixel { r: 1, g: 1, b: 1, a: 1 };
    const PIX2: RgbaPixel = RgbaPixel { r: 2, g: 2, b: 2, a: 2 };
    const PIX3: RgbaPixel = RgbaPixel { r: 3, g: 3, b: 3, a: 3 };
    const PIX4: RgbaPixel = RgbaPixel { r: 4, g: 4, b: 4, a: 4 };

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
                'x' => PIXEL_BLANK,
                '1' => PIX1,
                '2' => PIX2,
                '3' => PIX3,
                '4' => PIX4,
                c => panic!("Found undefined char in map: {}", c),
            };
            let c = match pixel_slice {
                PIXEL_BLANK => 'x',
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
        let textured = p.create_object_from_texture_exact(
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
        let textured = p.create_object_from_texture_exact(
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
    fn getting_pixel_from_object_at_position_stretched_works() {
        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );
        let textured = p.create_object_from_texture(
            0, Rect { x: 2, y: 1, w: 4, h: 4 },
            texture_from(&[PIX1, PIX2, PIX3, PIX4]),
            2
        );
        p.set_object_render_mode(textured, ObjectRenderMode::RenderToFit);
        p.draw_all_layers();
        let assert_map = [
            'x', 'x', 'x', 'x', 'x', 'x',
            'x', 'x', '1', '1', '2', '2',
            'x', 'x', '1', '1', '2', '2',
            'x', 'x', '3', '3', '4', '4',
            'x', 'x', '3', '3', '4', '4',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 6);

        let pixel = p.get_pixel_from_object_at(textured, 3, 2).unwrap();
        assert_eq!(pixel, PIX1);
        let pixel = p.get_pixel_from_object_at(textured, 4, 2).unwrap();
        assert_eq!(pixel, PIX2);
        let pixel = p.get_pixel_from_object_at(textured, 3, 3).unwrap();
        assert_eq!(pixel, PIX3);
        let pixel = p.get_pixel_from_object_at(textured, 4, 3).unwrap();
        assert_eq!(pixel, PIX4);

        // that works for an easy 1-2 scale, but what about a weird scale?
        // test if can be stretched big weird ratio
        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );
        // here we have a 3x2 image, stretched to a 4x4 bounds
        // so vertically the stretch is even, but horizontally
        // one of the columns will be wider than the others
        let textured = p.create_object_from_texture(
            0, Rect { x: 2, y: 1, w: 4, h: 4 },
            texture_from(&[PIX1, PIX2, PIX3, PIX4, PIXEL_BLUE, PIXEL_GREEN]),
            3
        );
        p.set_object_render_mode(textured, ObjectRenderMode::RenderToFit);
        p.draw_all_layers();
        let assert_map = [
            'x', 'x', 'x', 'x', 'x', 'x',
            'x', 'x', '1', '2', '3', '3',
            'x', 'x', '1', '2', '3', '3',
            'x', 'x', '4', 'b', 'g', 'g',
            'x', 'x', '4', 'b', 'g', 'g',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 6);

        println!("Here:");
        let pixel = p.get_pixel_from_object_at(textured, 2, 1).unwrap();
        assert_eq!(pixel, PIX1);
        let pixel = p.get_pixel_from_object_at(textured, 3, 1).unwrap();
        assert_eq!(pixel, PIX2);
        let pixel = p.get_pixel_from_object_at(textured, 4, 1).unwrap();
        assert_eq!(pixel, PIX3);
        let pixel = p.get_pixel_from_object_at(textured, 5, 1).unwrap();
        assert_eq!(pixel, PIX3);

        let pixel = p.get_pixel_from_object_at(textured, 2, 2).unwrap();
        assert_eq!(pixel, PIX1);
        let pixel = p.get_pixel_from_object_at(textured, 3, 2).unwrap();
        assert_eq!(pixel, PIX2);
        let pixel = p.get_pixel_from_object_at(textured, 4, 2).unwrap();
        assert_eq!(pixel, PIX3);
        let pixel = p.get_pixel_from_object_at(textured, 5, 2).unwrap();
        assert_eq!(pixel, PIX3);

        let pixel = p.get_pixel_from_object_at(textured, 2, 3).unwrap();
        assert_eq!(pixel, PIX4);
        let pixel = p.get_pixel_from_object_at(textured, 3, 3).unwrap();
        assert_eq!(pixel, PIXEL_BLUE);
        let pixel = p.get_pixel_from_object_at(textured, 4, 3).unwrap();
        assert_eq!(pixel, PIXEL_GREEN);
        let pixel = p.get_pixel_from_object_at(textured, 5, 3).unwrap();
        assert_eq!(pixel, PIXEL_GREEN);

        let pixel = p.get_pixel_from_object_at(textured, 2, 4).unwrap();
        assert_eq!(pixel, PIX4);
        let pixel = p.get_pixel_from_object_at(textured, 3, 4).unwrap();
        assert_eq!(pixel, PIXEL_BLUE);
        let pixel = p.get_pixel_from_object_at(textured, 4, 4).unwrap();
        assert_eq!(pixel, PIXEL_GREEN);
        let pixel = p.get_pixel_from_object_at(textured, 5, 4).unwrap();
        assert_eq!(pixel, PIXEL_GREEN);
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

    #[test]
    fn render_mode_as_much_as_possible_works() {
        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );
        // we purposefully dont give a 4th pixel
        // to test if the rendering works
        let textured = p.create_object_from_texture_exact(
            0, Rect { x: 2, y: 1, w: 2, h: 2 },
            texture_from(&[PIX1, PIX2, PIX3]),
        );

        // because its render as much as possible,
        // the last pixel should not be rendered because the texture
        // doesnt have enough pixels
        p.set_object_render_mode(textured, ObjectRenderMode::RenderAsMuchAsPossible);
        p.draw_all_layers();
        let assert_map = [
            'x', 'x', 'x', 'x',
            'x', 'x', '1', '2',
            'x', 'x', '3', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);

        // now if we instead have a texture that is bigger
        // than the bounds of the object, then it should simply
        // ignore the pixels after it reaches the end of the bounds

        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );
        let textured = p.create_object_from_texture(
            0, Rect { x: 2, y: 1, w: 2, h: 2 },
            texture_from(&[
                PIX1, PIX2, PIX3,
                PIX4, PIXEL_BLUE, PIXEL_BLUE,
                PIXEL_BLUE, PIXEL_BLUE, PIXEL_BLUE,
            ]),
            3,
        );
        // because its render as much as possible,
        // the pixels after the bounds should not exist
        p.set_object_render_mode(textured, ObjectRenderMode::RenderAsMuchAsPossible);
        p.draw_all_layers();
        let assert_map = [
            'x', 'x', 'x', 'x', 'x',
            'x', 'x', '1', '2', 'x',
            'x', 'x', '3', '4', 'x',
            'x', 'x', 'x', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 5);
    }

    #[test]
    fn render_mode_fit_works() {
        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );

        // first test if it can be stretched big
        let textured = p.create_object_from_texture(
            0, Rect { x: 2, y: 1, w: 4, h: 4 },
            texture_from(&[PIX1, PIX2, PIX3, PIX4]),
            2
        );
        p.set_object_render_mode(textured, ObjectRenderMode::RenderToFit);
        p.draw_all_layers();
        let assert_map = [
            'x', 'x', 'x', 'x', 'x', 'x',
            'x', 'x', '1', '1', '2', '2',
            'x', 'x', '1', '1', '2', '2',
            'x', 'x', '3', '3', '4', '4',
            'x', 'x', '3', '3', '4', '4',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 6);

        // test if can be stretched big weird ratio
        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );
        // here we have a 3x2 image, stretched to a 4x4 bounds
        // so vertically the stretch is even, but horizontally
        // one of the columns will be wider than the others
        let textured = p.create_object_from_texture(
            0, Rect { x: 2, y: 1, w: 4, h: 4 },
            texture_from(&[PIX1, PIX2, PIX3, PIX4, PIXEL_BLUE, PIXEL_GREEN]),
            3
        );
        p.set_object_render_mode(textured, ObjectRenderMode::RenderToFit);
        p.draw_all_layers();
        let assert_map = [
            'x', 'x', 'x', 'x', 'x', 'x',
            'x', 'x', '1', '2', '3', '3',
            'x', 'x', '1', '2', '3', '3',
            'x', 'x', '4', 'b', 'g', 'g',
            'x', 'x', '4', 'b', 'g', 'g',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 6);

        // test if can be stretched big weird ratio vertically
        let mut p = PortionRenderer::new(
            10, 10, 10, 10
        );
        // here we have a 2x3 image, stretched to a 4x4 bounds
        // so horizontally the stretch is even, but vertically
        // one of the rows will be taller than the others
        let textured = p.create_object_from_texture(
            0, Rect { x: 2, y: 1, w: 4, h: 4 },
            texture_from(&[PIX1, PIX2, PIX3, PIX4, PIXEL_BLUE, PIXEL_GREEN]),
            2
        );
        p.set_object_render_mode(textured, ObjectRenderMode::RenderToFit);
        p.draw_all_layers();
        let assert_map = [
            'x', 'x', 'x', 'x', 'x', 'x',
            'x', 'x', '1', '1', '2', '2',
            'x', 'x', '3', '3', '4', '4',
            'x', 'x', 'b', 'b', 'g', 'g',
            'x', 'x', 'b', 'b', 'g', 'g',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 6);
    }
}
