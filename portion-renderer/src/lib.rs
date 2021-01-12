use std::ops::Index;
use projection::ComputePoint;

pub mod portioner;
pub mod projection;
pub mod transform;
pub mod bounds;
pub use projection::Matrix;
pub use projection::RotateMatrix;
pub use transform::*;
pub use portioner::*;
pub use bounds::*;
pub use tightvec::TightVec;

#[macro_export]
macro_rules! get_red_index {
    ($x:expr, $y:expr, $w:expr, $indices_per_pixel:expr) => {
        $y * ($w * $indices_per_pixel) + ($x * $indices_per_pixel)
    };
}

#[macro_export]
macro_rules! get_pixel_start {
    ($x:expr, $y:expr, $pitch:expr, $indices_per_pixel:expr) => {
        $y * $pitch + ($x * $indices_per_pixel)
    }
}

pub const PIXEL_BLANK: RgbaPixel = RgbaPixel { r: 0, g: 0, b: 0, a: 0 };
pub const PIXEL_BLACK: RgbaPixel = RgbaPixel { r: 0, g: 0, b: 0, a: 255 };
pub const PIXEL_RED: RgbaPixel = RgbaPixel { r: 255, g: 0, b: 0, a: 255 };
pub const PIXEL_GREEN: RgbaPixel = RgbaPixel { r: 0, g: 255, b: 0, a: 255 };
pub const PIXEL_BLUE: RgbaPixel = RgbaPixel { r: 0, g: 0, b: 255, a: 255 };

// indices per pixel
pub const ABGR8888_IPP: u32 = 4;
pub const ARGB8888_IPP: u32 = 4;
pub const RGBA8888_IPP: u32 = 4;
pub const BGRA8888_IPP: u32 = 4;
pub const RGBA32_IPP: u32 = 1;

static EMPTY_OBJECT: Object = Object {
    previous_bounds: EMPTY_RECT, current_bounds: EMPTY_RECT,
    layer_index: 0, texture_index: 0, initial_render: false,
    texture_color: None, transform: None,
};

pub struct PortionRenderer<T> {
    pixel_buffer: Vec<T>,
    clear_buffer: Vec<T>,
    portioner: Portioner,

    width: u32,
    height: u32,
    pitch: usize,
    pixel_format: PixelFormatEnum,
    indices_per_pixel: u32,

    textures: TightVec<Texture<T>>,
    layers: Vec<Layer>,
    objects: TightVec<Object>,
}

// TODO: actually use these.
// right now implementation just assumes RGBA8888....
pub enum PixelFormatEnum {
    ABGR8888,
    ARGB8888,
    RGBA8888,
    BGRA8888,
    RGBA32,
}

pub struct Layer {
    /// a human friendly index
    /// a Layer is stored in a vec where its actual index
    /// does not necessarily correspond to this index.
    /// this value just lets you easily create layers via:
    /// layer {index: 0}, layer {index: 10000}, layer {index: 500}, etc.
    pub index: u32,
    /// a vector of objects indices that exist on this layer
    /// you can get the object via Renderer.objects[Layer.objects[...]]
    pub objects: Vec<usize>,
    /// a vector of objects indices on this layer that need to be updated next render cycle
    /// you can get the objects via Renderer.objects[Layer.objects[...]]
    pub updates: Vec<usize>,
}

#[derive(Clone)]
pub struct Texture<T> {
    pub data: Vec<T>,
    pub width: u32,
    pub height: u32,
}

#[derive(Copy, Clone)]
pub struct Transform {
    pub matrix: Matrix,
    pub bounds: TiltedRect,
}

#[derive(Clone)]
pub struct Object {
    pub texture_color: Option<RgbaPixel>,
    pub texture_index: usize,
    pub transform: Option<Transform>,
    pub layer_index: usize,
    pub current_bounds: Rect,
    pub previous_bounds: Rect,
    pub initial_render: bool,
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

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct RgbaPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub trait SetPixel<T> {
    fn set_pixel(&mut self, pixel: &[T]);
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

impl GetRectangularBounds for Object {
    #[inline(always)]
    fn get_bounds(&self) -> Rect {
        match self.transform {
            Some(transform) => transform.bounds.get_bounds(),
            None => self.current_bounds,
        }
    }
}

impl SetPixel<u8> for &mut [u8] {
    #[inline(always)]
    fn set_pixel(&mut self, pixel: &[u8]) {
        self[0] = pixel[0];
        self[1] = pixel[1];
        self[2] = pixel[2];
        self[3] = pixel[3];
    }
}

impl PixelFormatEnum {
    #[inline(always)]
    pub fn indices_per_pixel(&self) -> u32 {
        match self {
            PixelFormatEnum::ABGR8888 => ABGR8888_IPP,
            PixelFormatEnum::ARGB8888 => ARGB8888_IPP,
            PixelFormatEnum::RGBA8888 => RGBA8888_IPP,
            PixelFormatEnum::BGRA8888 => BGRA8888_IPP,
            PixelFormatEnum::RGBA32 => RGBA32_IPP,
        }
    }
}

impl<'a> Default for Object {
    fn default() -> Self {
        EMPTY_OBJECT.clone()
    }
}

impl Layer {
    /// returns the layer's actual index of the Vec its in,
    /// whereas the layer_index: u32 is a human friendly index
    /// like 0, 1000, 1001, etc.
    pub fn get_or_make_layer(layers: &mut Vec<Layer>, layer_index: u32) -> usize {
        let mut insert_at_index = 0;
        let mut update_at_index = None;
        let last_i = layers.len() - 1;
        for (i, layer) in layers.iter().enumerate() {
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
            layers.push(Layer {
                index: layer_index,
                objects: vec![],
                updates: vec![],
            });
            insert_at_index
        }
    }
}

/// Dont use this in a real program
/// this is just convenient for debugging
/// the issue with using this in a real program is its not very
/// performant if you are accessing many pixels at once
/// because the indices (x, y) are probably not cached
/// between calls to the .index((x, y)) method
impl<T> Index<(u32, u32)> for PortionRenderer<T> {
    type Output = [T];

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

impl<T> AsRef<Portioner> for PortionRenderer<T> {
    fn as_ref(&self) -> &Portioner { &self.portioner }
}
impl<T> AsMut<Portioner> for PortionRenderer<T> {
    fn as_mut(&mut self) -> &mut Portioner { &mut self.portioner }
}
impl<T> AsMut<Vec<T>> for PortionRenderer<T> {
    fn as_mut(&mut self) -> &mut Vec<T> { &mut self.pixel_buffer }
}

/// constructors, and some other methods
/// requires T to have a default, which T should be
/// either u32 or u8,
impl<T: Default + Clone> PortionRenderer<T> {
    /// provides sensible default of 4x4 portion grid,
    /// and RGBA8888 pixel format. if you dont like these defaults,
    /// use new_ex instead and manually set your starting parameters
    pub fn new(
        width: u32,
        height: u32,
    ) -> PortionRenderer<T> {
        PortionRenderer::new_ex(width, height, 4, 4, PixelFormatEnum::RGBA8888)
    }

    pub fn new_ex(
        width: u32,
        height: u32,
        num_rows: u32,
        num_cols: u32,
        pixel_format: PixelFormatEnum,
    ) -> PortionRenderer<T> {
        let indices_per_pixel = pixel_format.indices_per_pixel();
        let num_pixels = width * height;
        let data_len: usize = (num_pixels * indices_per_pixel) as usize;
        let pixel_buffer = vec![T::default(); data_len];
        let pitch = (width * indices_per_pixel) as usize;
        PortionRenderer {
            clear_buffer: pixel_buffer.clone(),
            pixel_buffer,
            width,
            pitch,
            height,
            indices_per_pixel,
            pixel_format,
            layers: vec![Layer { index: 0, objects: vec![], updates: vec![], }],
            textures: TightVec::new(),
            objects: TightVec::new(),
            portioner: Portioner::new(width, height, num_rows, num_cols),
        }
    }

    /// clones the current visible buffer to the clear buffer
    /// useful when you want to render an intial scene, and
    /// then use that as the background
    pub fn set_clear_buffer(&mut self) {
        self.clear_buffer = self.pixel_buffer.clone();
    }
}

impl<T> PortionRenderer<T> {
    /// returns the layer's actual index of the Vec its in,
    /// whereas the layer_index: u32 is a human friendly index
    /// like 0, 1000, 1001, etc.
    pub fn get_or_make_layer(&mut self, layer_index: u32) -> usize {
        Layer::get_or_make_layer(&mut self.layers, layer_index)
    }

    pub fn set_object_updated(&mut self, object_index: usize) {
        let layer_index = self.objects[object_index].layer_index;
        self.set_object_updated_on_layer(object_index, layer_index)
    }

    fn set_object_updated_on_layer(&mut self, object_index: usize, layer_index: usize) {
        self.layers[layer_index].objects.push(object_index);
        self.layers[layer_index].updates.push(object_index);
    }

    pub fn create_object(
        &mut self, layer_index: u32, bounds: Rect,
        texture: Option<Texture<T>>,
        color: Option<RgbaPixel>,
    ) -> usize {
        let texture_index = if let Some(txt) = texture {
            self.textures.insert(txt)
        } else { 0 };
        let layer_index = self.get_or_make_layer(layer_index);
        let new_object = Object {
            texture_color: color,
            transform: None,
            layer_index,
            texture_index,
            current_bounds: bounds,
            previous_bounds: bounds,
            initial_render: true,
        };
        let new_object_index = self.objects.insert(new_object);
        self.set_object_updated_on_layer(new_object_index, layer_index);
        new_object_index
    }

    pub fn create_object_from_color(
        &mut self, layer_index: u32, bounds: Rect,
        color: RgbaPixel
    ) -> usize {
        self.create_object(layer_index, bounds, None, Some(color))
    }

    pub fn create_object_from_texture(
        &mut self, layer_index: u32, bounds: Rect,
        texture: Vec<T>, texture_width: u32, texture_height: u32,
    ) -> usize {
        let texture = Texture {
            data: texture,
            width: texture_width,
            height: texture_height,
        };
        self.create_object(layer_index, bounds, Some(texture), None)
    }

    /// unlike `create_object_from_texture`, this method assumes that the bounds of the object
    /// being created are exactly the same as the bounds of the texture vec being passed in.
    /// it is your responsibility as the user to ensure that:
    /// bounds.w * bounds.h = texture.len() * indices_per_pixel
    /// where the indices_per_pixel is the same as what the renderer is using.
    /// eg: if using pixel format RGBA8888, and a bounds.w and bounds.h == 2, then
    /// the texture vec should be 2 * 2 * 4 = 16 elements long.
    pub fn create_object_from_texture_exact(
        &mut self, layer_index: u32, bounds: Rect,
        texture: Vec<T>
    ) -> usize {
        self.create_object_from_texture(layer_index, bounds, texture, bounds.w, bounds.h)
    }

    pub fn object_needs_drawing(&mut self, object_index: usize) -> bool {
        let object = &self.objects[object_index];
        object.previous_bounds != object.current_bounds
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
        let object_current_bounds = &self.objects[object_index].get_bounds();
        let object_previous_bounds = &self.objects[object_index].previous_bounds;
        let mut above_bounds = AboveRegions::default();
        for i in start_layer_check_at..layers {
            let layer = &self.layers[i];
            for layer_object_index in layer.objects.iter() {
                let layer_object = &self.objects[*layer_object_index];
                if let Some(intersection) = layer_object.get_bounds().intersection(*object_current_bounds) {
                    above_bounds.above_my_current.push(intersection);
                }
                if let Some(intersection) = layer_object.get_bounds().intersection(*object_previous_bounds) {
                    above_bounds.above_my_previous.push(intersection);
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
        let object_previous_bounds = &self.objects[object_index].previous_bounds;
        let mut below_bounds = BelowRegions::default();
        let start_layer_check_at = layer_index;
        for i in (0..start_layer_check_at).rev() {
            let layer = &self.layers[i];
            for layer_object_index in layer.objects.iter() {
                let layer_object = &self.objects[*layer_object_index];
                if let Some(intersection) = layer_object.get_bounds().intersection(*object_previous_bounds) {
                    below_bounds.below_my_previous.push(BelowRegion {
                        region: intersection,
                        region_belongs_to: *layer_object_index,
                    });
                }
            }
        }

        below_bounds
    }

    pub fn set_object_rotation(&mut self, object_index: usize, degrees: f32) {
        if degrees == 0f32 {
            if self.objects[object_index].transform.is_some() {
                self.objects[object_index].transform = None;
                self.set_layer_update(object_index);
            }
            return;
        }

        let current_bounds = self.objects[object_index].current_bounds;
        let transform_matrix = Matrix::rotate_degrees(degrees);
        let inverse_transform = transform_matrix.invert().unwrap();
        let tilted_rect = TiltedRect::from_bounds_and_matrix(current_bounds, transform_matrix);
        let t = Transform {
            matrix: inverse_transform,
            bounds: tilted_rect,
        };
        self.objects[object_index].transform = Some(t);
        self.set_layer_update(object_index);
    }

    pub fn set_layer_update(&mut self, object_index: usize) {
        let layer_index = self.objects[object_index].layer_index;
        self.layers[layer_index].updates.push(object_index);
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
        if let Some(transform) = &mut self.objects[object_index].transform {
            transform.bounds.shift_bounds_x(by);
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
        if let Some(transform) = &mut self.objects[object_index].transform {
            transform.bounds.shift_bounds_y(by);
        }
    }
}


/// This is the implementation for any pixel format in 8888 format
/// TODO: implement these methods for 32 format
impl PortionRenderer<u8> {
    pub fn draw(&mut self, pixels: &[u8], bounds: Rect) {
        let x = bounds.x as usize;
        let y = bounds.y as usize;
        let w = bounds.w as usize;
        let h = bounds.h as usize;
        let self_width = self.width as usize;
        let indices_per_pixel = self.indices_per_pixel as usize;
        let mut pixels_index = 0;
        for i in y..(y + h) {
            for j in x..(x + w) {
                let red_index = get_red_index!(j, i, self_width, indices_per_pixel);
                let next_index = red_index + indices_per_pixel;
                unsafe {
                    let mut dest_pixel = self.pixel_buffer.get_unchecked_mut(red_index..next_index);
                    let src_pixel = pixels.get_unchecked(pixels_index..pixels_index + indices_per_pixel);
                    dest_pixel.set_pixel(src_pixel);
                }

                pixels_index += 4;
            }
        }
    }

    pub fn get_pixel_from_object_at_rotated(
        &self,
        object_index: usize,
        transform: &Transform,
        x: u32, y: u32,
    ) -> Option<RgbaPixel> {
        let transform_matrix: RotateMatrix = (&transform.matrix).into();
        let (shift_x, shift_y, texture_width, texture_height, texture_data) = {
            let obj = &self.objects[object_index];
            let texture_index = obj.texture_index;
            let texture = &self.textures[texture_index];
            let cb = &obj.current_bounds;
            (cb.x as f32, cb.y as f32, texture.width, texture.height, &texture.data)
        };
        let x_shift = x as f32 - shift_x;
        let y_shift = y as f32 - shift_y;
        let (px, py) = transform_matrix.compute_pt(x_shift, y_shift);
        let pix = interpolate_nearest(
            &texture_data, texture_width, texture_height,
            px, py, PIXEL_BLANK
        );
        Some(pix)
    }

    pub fn get_pixel_from_object_at(
        &self,
        object_index: usize,
        x: u32, y: u32
    ) -> Option<RgbaPixel> {
        if let Some(transform) = &self.objects[object_index].transform {
            return self.get_pixel_from_object_at_rotated(object_index, transform, x, y);
        }

        if let Some(color) = self.objects[object_index].texture_color {
            return Some(color);
        }

        let texture_index = self.objects[object_index].texture_index;
        let texture = &self.textures[texture_index];

        let current_bounds = self.objects[object_index].current_bounds;
        // it should be guaranteed that x and y exist within the objects current bounds
        if x < current_bounds.x || y < current_bounds.y {
            panic!("Called get_pixel_from_object_at with ({}, {}) but objects bounds are {:?}", x, y, current_bounds);
        }

        // TODO: what if the object has a matrix transormation?
        // need to handle that here to get the pixel value after transform
        // currently this assumes the objects bounds are the same as the texture bounds!
        let local_x = x - current_bounds.x;
        let local_y = y - current_bounds.y;
        let red_index = get_red_index!(local_x, local_y, current_bounds.w, self.indices_per_pixel) as usize;
        let pixel: RgbaPixel = match texture.data.get(red_index..(red_index+4)) {
            Some(u8_slice) => u8_slice.into(),
            None => return None,
        };
        Some(pixel)
    }

    pub fn clear_pixels_from_below_object(&mut self, pb_red_index: usize, x: u32, y: u32, skip_below: &BelowRegions) -> bool {
        for below in skip_below.below_my_previous.iter() {
            if below.region.contains_u32(x, y) {
                let pixel = self.get_pixel_from_object_at(
                    below.region_belongs_to, x, y
                );
                if let Some(pixel) = pixel {
                    if pixel.a == 0 {
                        return false;
                    }

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

    pub fn draw_all_layers(&mut self) {
        // TODO: can we avoid drawing bottom layers
        // if a top layer fully covers it up?
        let mut draw_object_indices = vec![];
        for (layer_index, layer) in self.layers.iter_mut().enumerate() {
            // make sure to drain so we remove these updates
            // and prevent them from showing up next draw
            for object_index in layer.updates.drain(..) {
                draw_object_indices.push((layer_index, object_index));
            }
        }

        for (layer_index, object_index) in draw_object_indices {
            let above_regions = self.get_regions_above_object(object_index, layer_index);
            let below_regions = self.get_regions_below_object(object_index, layer_index);
            self.draw_object(object_index, above_regions, below_regions);
        }
    }

    /// like draw_all_layers, but iterates over layer.objects instead of
    /// layer.updates, so it will always draw every object on every layer
    /// mostly used for testing/benchmarking
    pub fn force_draw_all_layers(&mut self) {
        let mut draw_object_indices = vec![];
        for (layer_index, layer) in self.layers.iter_mut().enumerate() {
            for object_index in layer.objects.iter() {
                draw_object_indices.push((layer_index, *object_index));
            }
        }

        for (layer_index, object_index) in draw_object_indices {
            let above_regions = self.get_regions_above_object(object_index, layer_index);
            let below_regions = self.get_regions_below_object(object_index, layer_index);
            self.draw_object(object_index, above_regions, below_regions);
        }
    }

    pub fn draw_pixel(
        &mut self, pixel: RgbaPixel,
        skip_above: AboveRegions,
        transform: Option<Transform>,
        min_y: u32, max_y: u32,
        min_x: u32, max_x: u32,
        width: u32,
        height: u32,
    ) {
        if let Some(transform) = transform {
            let transform_bounds = transform.bounds.get_bounds();
            let tmin_x = transform_bounds.x;
            let tmax_x = tmin_x + transform_bounds.w;
            let tmin_y = transform_bounds.y;
            let tmax_y = tmin_y + transform_bounds.h;
            return self.draw_pixel_rotated(pixel,
                &skip_above, transform.matrix,
                tmin_y, tmax_y,
                tmin_x, tmax_x,
                min_x as f32,
                min_y as f32,
                width, height
            );
        }

        for i in min_y..max_y {
            for j in min_x..max_x {
                if should_skip_point(&skip_above.above_my_current, j, i) {
                    continue;
                }
                self.portioner.take_pixel(j, i);
                let red_index = get_red_index!(j, i, self.width, self.indices_per_pixel);
                let red_index = red_index as usize;
                // TODO: pixel format???
                self.pixel_buffer[red_index] = pixel.r;
                self.pixel_buffer[red_index + 1] = pixel.g;
                self.pixel_buffer[red_index + 2] = pixel.b;
                self.pixel_buffer[red_index + 3] = pixel.a;
            }
        }
    }

    pub fn draw_pixel_rotated(
        &mut self, pixel: RgbaPixel,
        skip_above: &AboveRegions,
        transform: Matrix,
        min_y: u32, max_y: u32,
        min_x: u32, max_x: u32,
        shift_x: f32, shift_y: f32,
        width: u32, height: u32,
    ) {
        let transform: RotateMatrix = (&transform).into();
        for i in min_y..max_y {
            for j in min_x..max_x {
                if should_skip_point(&skip_above.above_my_current, j, i) {
                    continue;
                }
                self.portioner.take_pixel(j, i);
                let j_shift = j as f32 - shift_x;
                let i_shift = i as f32 - shift_y;
                let (px, py) = transform.compute_pt(j_shift, i_shift);
                let pix = interpolate_nearest_pixel(
                    pixel, width, height,
                    px, py, PIXEL_BLANK
                );
                if pix.a == 0 {
                    continue;
                }
                // println!("({}, {}), [{}, {}] => GOT PIXEL: {:?}", j, i, px, py, pix);
                let red_index = get_red_index!(j, i, self.width, self.indices_per_pixel);
                let red_index = red_index as usize;
                // TODO: pixel format?
                self.pixel_buffer[red_index] = pix.r;
                self.pixel_buffer[red_index + 1] = pix.g;
                self.pixel_buffer[red_index + 2] = pix.b;
                self.pixel_buffer[red_index + 3] = pix.a;
            }
        }
    }

    pub fn draw_exact_rotated(
        &mut self, texture_index: usize,
        skip_above: &AboveRegions,
        transform: Matrix,
        min_y: u32, max_y: u32,
        min_x: u32, max_x: u32,
        shift_x: f32, shift_y: f32,
    ) {
        let transform: RotateMatrix = (&transform).into();
        let texture = &self.textures[texture_index];
        let texture_data = &texture.data;
        let texture_width = texture.width;
        let texture_height = texture.height;
        for i in min_y..max_y {
            for j in min_x..max_x {
                if should_skip_point(&skip_above.above_my_current, j, i) {
                    continue;
                }
                self.portioner.take_pixel(j, i);
                let j_shift = j as f32 - shift_x;
                let i_shift = i as f32 - shift_y;
                let (px, py) = transform.compute_pt(j_shift, i_shift);
                let pix = interpolate_nearest(
                    texture_data, texture_width, texture_height,
                    px, py, PIXEL_BLANK
                );
                if pix.a == 0 {
                    continue;
                }
                // println!("({}, {}), [{}, {}] => GOT PIXEL: {:?}", j, i, px, py, pix);
                let red_index = get_red_index!(j, i, self.width, self.indices_per_pixel);
                let red_index = red_index as usize;
                // TODO: pixel format?
                self.pixel_buffer[red_index] = pix.r;
                self.pixel_buffer[red_index + 1] = pix.g;
                self.pixel_buffer[red_index + 2] = pix.b;
                self.pixel_buffer[red_index + 3] = pix.a;
            }
        }
    }

    pub fn draw_exact(
        &mut self, texture_index: usize,
        skip_above: AboveRegions,
        transform: Option<Transform>,
        min_y: u32, max_y: u32,
        min_x: u32, max_x: u32,
    ) {
        if let Some(transform) = transform {
            let transform_bounds = transform.bounds.get_bounds();
            let tmin_x = transform_bounds.x;
            let tmax_x = tmin_x + transform_bounds.w;
            let tmin_y = transform_bounds.y;
            let tmax_y = tmin_y + transform_bounds.h;
            return self.draw_exact_rotated(texture_index,
                &skip_above, transform.matrix,
                tmin_y, tmax_y,
                tmin_x, tmax_x,
                min_x as f32,
                min_y as f32,
            );
        }

        let item_pixels = &self.textures[texture_index].data;
        let indices_per_pixel = self.indices_per_pixel as usize;
        let mut item_pixel_index = 0;
        for i in min_y..max_y {
            for j in min_x..max_x {
                // if the alpha value is 0, skip this pixel
                if item_pixels[item_pixel_index + 3] == 0 {
                    item_pixel_index += indices_per_pixel;
                    continue;
                }
                if should_skip_point(&skip_above.above_my_current, j, i) {
                    item_pixel_index += indices_per_pixel;
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
            }
        }
    }

    pub fn clear_object_previous_bounds(
        &mut self,
        skip_above: &AboveRegions,
        skip_below: &BelowRegions,
        min_y: u32, max_y: u32,
        min_x: u32, max_x: u32,
    ) {
        let should_try_clear_below = !skip_below.below_my_previous.is_empty();
        for i in min_y..max_y {
            for j in min_x..max_x {
                if should_skip_point(&skip_above.above_my_previous, j, i) {
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

                self.pixel_buffer[red_index] = self.clear_buffer[red_index];
                self.pixel_buffer[red_index + 1] = self.clear_buffer[red_index + 1];
                self.pixel_buffer[red_index + 2] = self.clear_buffer[red_index + 2];
                self.pixel_buffer[red_index + 3] = self.clear_buffer[red_index + 3];
            }
        }
    }

    pub fn draw_object(&mut self, object_index: usize, skip_above: AboveRegions, skip_below: BelowRegions) {
        let (
            previous_bounds, is_first_time, texture_index, object_color,
        ) = {
            let object = &self.objects[object_index];
            (object.previous_bounds, object.initial_render, object.texture_index, object.texture_color)
        };
        let prev_x = previous_bounds.x;
        let prev_y = previous_bounds.y;
        let prev_w = previous_bounds.w;
        let prev_h = previous_bounds.h;
        if !is_first_time {
            self.clear_object_previous_bounds(
                &skip_above,
                &skip_below,
                prev_y, prev_y + prev_h,
                prev_x, prev_x + prev_w,
            );
        } else {
            self.objects[object_index].initial_render = false;
        }

        let [
            now_x, now_y,
            now_w, now_h,
        ] = {
            let object = &self.objects[object_index];
            let now = object.current_bounds;
            [now.x, now.y, now.w, now.h]
        };

        if let Some(color) = object_color {
            // can skip rendering if the alpha is 0, no point in iterating
            if color.a == 0 {
                let mut object = &mut self.objects[object_index];
                object.previous_bounds = object.get_bounds();
                return;
            }
            self.draw_pixel(color, skip_above,
                self.objects[object_index].transform,
                now_y, now_y + now_h,
                now_x, now_x + now_w,
                now_w, now_h,
            );
        } else {
            self.draw_exact(
                texture_index, skip_above,
                self.objects[object_index].transform,
                now_y, now_y + now_h,
                now_x, now_x + now_w
            );
        }

        let mut object = &mut self.objects[object_index];
        object.previous_bounds = object.get_bounds();
    }

    pub fn draw_grid_outline(&mut self) {
        draw_grid_outline(&self.portioner, &mut self.pixel_buffer, self.indices_per_pixel);
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

    fn assert_pixels_in_map(p: &mut PortionRenderer<u8>, map: &[char], width: u32) {
        const IDC_PIXEL: RgbaPixel = RgbaPixel { r: 22, g: 103, b: 2, a: 54 };
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
                '?' => IDC_PIXEL,
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
            let c = if pixel_compare == IDC_PIXEL { '?' } else { c };
            actual_string.push(c);
            actual_string.push_str(", ");
            if should_newline {
                actual_string.push_str("\n ");
            }
            if pixel_compare != IDC_PIXEL && pixel_compare != pixel_slice {
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

    fn get_test_renderer() -> PortionRenderer<u8> {
        PortionRenderer::new_ex(
            10, 10, 10, 10, PixelFormatEnum::RGBA8888
        )
    }

    #[test]
    fn managed_layering_works() {
        let mut p = PortionRenderer::<u8>::new_ex(
            10, 10, 10, 10, PixelFormatEnum::RGBA8888,
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

    #[test]
    fn draw_arbitrary_bound_works() {
        // test that you can render an arbitrary pixel vec
        // in an area as given by the bounds
        let mut p = get_test_renderer();
        let mut pixels = vec![255; 9 * 9 * 4];
        // red
        pixels[0] = 255;
        pixels[1] = 0;
        pixels[2] = 0;
        pixels[3] = 255;
        // blue
        pixels[4] = 0;
        pixels[5] = 0;
        pixels[6] = 255;
        pixels[7] = 255;
        p.draw(&pixels, Rect { x: 1, y: 1, w: 9, h: 9 });
        let assert_map = [
            '?', '?', '?',
            '?', 'r', 'b',
            '?', '?', '?',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 3);
    }

    #[test]
    fn simple_texture_move_works() {
        let mut p = get_test_renderer();
        let t = p.create_object_from_texture(
            0, Rect { x: 0, y: 0, w: 2, h: 2 },
            texture_from(&[PIX1, PIX2, PIX3, PIX4]),
            2, 2,
        );
        p.draw_all_layers();
        let assert_map = [
            '1', '2', 'x', 'x',
            '3', '4', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);

        p.move_object_x_by(t, 1);
        p.draw_all_layers();
        let assert_map = [
            'x', '1', '2', 'x',
            'x', '3', '4', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 4);
    }

    #[test]
    fn getting_pixel_from_object_at_position_works() {
        let mut p = get_test_renderer();
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
    fn simple_overlap_works() {
        let mut p = get_test_renderer();
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
        let mut p = get_test_renderer();
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
        let mut p = get_test_renderer();
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
        let mut p = get_test_renderer();
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
        let mut p = get_test_renderer();
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
        let mut p = get_test_renderer();
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
        let mut p = get_test_renderer();
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
    fn default_render_mode_for_textures_works() {
        let mut p = get_test_renderer();
        // if we have a texture that is bigger
        // than the bounds of the object, then it should simply
        // ignore the pixels after it reaches the end of the bounds
        let _ = p.create_object_from_texture(
            0, Rect { x: 2, y: 1, w: 2, h: 2 },
            texture_from(&[
                PIX1, PIX2, PIX3,
                PIX4, PIXEL_BLUE, PIXEL_BLUE,
                PIXEL_BLUE, PIXEL_BLUE, PIXEL_BLUE,
            ]),
            3, 3,
        );
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
    fn can_draw_arbitrary_rotations1() {
        let mut p = get_test_renderer();
        let t = p.create_object_from_texture(
            0, Rect { x: 2, y: 1, w: 2, h: 2 },
            texture_from(&[
                PIX1, PIX2,
                PIX3, PIX4,
            ]),
            2, 2,
        );
        p.draw_all_layers();
        let assert_map = [
            'x', 'x', 'x', 'x', 'x',
            'x', 'x', '1', '2', 'x',
            'x', 'x', '3', '4', 'x',
            'x', 'x', 'x', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 5);

        p.set_object_rotation(t, -90f32);

        p.draw_all_layers();
        let assert_map = [
            'x', 'x', '2', '4', 'x',
            'x', 'x', '1', '3', 'x',
            'x', 'x', 'x', 'x', 'x',
            'x', 'x', 'x', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 5);

        p.move_object_x_by(t, -1);
        p.draw_all_layers();
        let assert_map = [
            'x', '2', '4', 'x', 'x',
            'x', '1', '3', 'x', 'x',
            'x', 'x', 'x', 'x', 'x',
            'x', 'x', 'x', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 5);

        // we undo the rotation and move back 1
        p.set_object_rotation(t, 0.0);
        p.move_object_x_by(t, 1);
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
    fn can_draw_arbitrary_rotations2() {
        // same idea as the other test, but we
        // want to check if it works for angles between 0-90
        let mut p = get_test_renderer();
        let t = p.create_object_from_texture(
            0, Rect { x: 2, y: 1, w: 2, h: 2 },
            texture_from(&[
                PIX1, PIX2,
                PIX3, PIX4,
            ]),
            2, 2,
        );
        p.draw_all_layers();
        let assert_map = [
            'x', 'x', 'x', 'x', 'x',
            'x', 'x', '1', '2', 'x',
            'x', 'x', '3', '4', 'x',
            'x', 'x', 'x', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 5);

        p.set_object_rotation(t, -45f32);

        p.draw_all_layers();
        let assert_map = [
            'x', 'x', 'x', '2', 'x',
            'x', 'x', '1', '?', '?',
            'x', 'x', 'x', '?', '?',
            'x', 'x', 'x', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 5);

        p.move_object_x_by(t, -1);
        p.draw_all_layers();
        let assert_map = [
            'x', 'x', '2', 'x', 'x',
            'x', '1', '?', '?', 'x',
            'x', 'x', '?', 'x', 'x',
            'x', 'x', 'x', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 5);

        // we undo the rotation and move back 1
        p.set_object_rotation(t, 0.0);
        p.move_object_x_by(t, 1);
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
    fn can_draw_arbitrary_rotations_for_solid_colors() {
        let mut p = get_test_renderer();
        let red = p.create_object_from_color(
            1, Rect { x: 2, y: 1, w: 2, h: 2 },
            PIXEL_RED
        );
        p.draw_all_layers();
        let assert_map = [
            'x', 'x', 'x', 'x', 'x',
            'x', 'x', 'r', 'r', 'x',
            'x', 'x', 'r', 'r', 'x',
            'x', 'x', 'x', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 5);

        p.set_object_rotation(red, -45f32);
        p.draw_all_layers();
        let assert_map = [
            'x', 'x', 'x', 'r', 'x',
            'x', 'x', 'r', '?', '?',
            'x', 'x', 'x', '?', '?',
            'x', 'x', 'x', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 5);

        p.move_object_x_by(red, -1);
        p.draw_all_layers();
        let assert_map = [
            'x', 'x', 'r', 'x', 'x',
            'x', 'r', '?', '?', 'x',
            'x', 'x', '?', 'x', 'x',
            'x', 'x', 'x', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 5);

        // we undo the rotation and move back 1
        p.set_object_rotation(red, 0.0);
        p.move_object_x_by(red, 1);
        p.draw_all_layers();
        let assert_map = [
            'x', 'x', 'x', 'x', 'x',
            'x', 'x', 'r', 'r', 'x',
            'x', 'x', 'r', 'r', 'x',
            'x', 'x', 'x', 'x', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 5);
    }

    // this is something we use for benchmarking, but I want to
    // make sure that what is being benchmarked is accurate
    // so this test case just checks that the rotation works,
    // and then shifts the rectangle towards the middle.
    #[test]
    fn benchmark_example() {
        let mut p = PortionRenderer::<u8>::new_ex(
            1000, 1000, 10, 10, PixelFormatEnum::RGBA8888
        );
        let red = p.create_object_from_color(
            1, Rect { x: 0, y: 0, w: 500, h: 400 },
            PIXEL_RED
        );
        p.set_object_rotation(red, 45f32);
        p.draw_all_layers();
        let assert_map = [
            'r', 'x', 'x', 'x', 'x',
            'r', 'r', 'x', 'x', 'x',
            'r', 'r', 'r', 'x', 'x',
            'r', 'r', 'r', 'r', 'x',
        ];
        assert_pixels_in_map(&mut p, &assert_map, 5);
        p.move_object_x_by(red, 200);
        p.draw_all_layers();
    }
}
