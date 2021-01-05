use std::f64::consts::PI;

use super::RgbaPixel;
use super::get_red_index;
use super::Matrix;

macro_rules! rotate_point {
    ($x:expr, $y:expr, $sin:expr, $cos:expr) => {
        (($x * $cos) - ($y * $sin), ($x * $sin) + ($y * $cos))
    };
}

pub fn min_f64(a: f64, b: f64, c: f64) -> f64 {
    let c = if c < 0.0 { c } else { 0.0 };
    let b = if b < c { b } else { c };
    let a = if a < b { a } else { b };
    a
}

pub fn max_f64(a: f64, b: f64, c: f64) -> f64 {
    let c = if c > 0.0 { c } else { 0.0 };
    let b = if b > c { b } else { c };
    let a = if a > b { a } else { b };
    a
}

pub fn rotated_size(width: usize, height: usize, angle: f64) -> (u32, u32) {
    if width <= 0 || height <= 0 {
        return (0, 0);
    }

    let width_f64 = width as f64;
    let height_f64 = height as f64;

    let radians  = PI * angle / 180.0;
    let (sin, cos) = (radians.sin(), radians.cos());

    let (x1, y1) = rotate_point!(width_f64 - 1.0, 0.0, sin, cos);
    let (x2, y2) = rotate_point!(width_f64 - 1.0, height_f64 - 1.0, sin, cos);
    let (x3, y3) = rotate_point!(0.0, height_f64 - 1.0, sin, cos);

    let min_x = min_f64(x1, x2, x3);
    let max_x = max_f64(x1, x2, x3);
    let min_y = min_f64(y1, y2, y3);
    let max_y = max_f64(y1, y2, y3);


    let new_width = max_x - min_x + 1.0;
    let new_width_floor = new_width.floor();
    let new_width = if new_width - new_width_floor > 0.1 {
        new_width_floor as u32 + 1
    } else {
        new_width_floor as u32
    };

    let new_height = max_y - min_y + 1.0;
    let new_height_floor = new_height.floor();
    let new_height = if new_height - new_height_floor > 0.1 {
        new_height_floor as u32 + 1
    } else {
        new_height_floor as u32
    };

    (new_width, new_height)
}


fn blend_bilinear(
    top_left: &[u8],
    top_right: &[u8],
    bottom_left: &[u8],
    bottom_right: &[u8],
    right_weight: f32,
    bottom_weight: f32,
) -> RgbaPixel {

    // merge top left and top right:
    // and merge bottom left and bottom right:
    let mut top = [0, 0, 0, 0];
    let mut bottom = [0, 0, 0, 0];
    for i in 0..3 {
        let something = (1f32 - right_weight) * top_left[i] as f32 + right_weight * top_right[i] as f32;
        let other = (1f32 - right_weight) * bottom_left[i] as f32 + right_weight * bottom_right[i] as f32;
        top[i] = something as u8;
        bottom[i] = other as u8;
    }
    println!("{:?}", top);
    println!("{:?}", bottom);

    // we want to be alpha: v
    let mut out = [0, 0, 0, 255];
    for i in 0..3 {
        let other = (1f32 - bottom_weight) * top[i] as f32 + bottom_weight * bottom[i] as f32;
        out[i] = other as u8;
    }

    return RgbaPixel {
        r: out[0],
        g: out[1],
        b: out[2],
        a: out[3],
    };
}


fn interpolate_bilinear(
    texture: &[u8],
    texture_width: u32,
    texture_height: u32,
    x: f32,
    y: f32,
    default: RgbaPixel
) -> RgbaPixel {
    let left = x.floor();
    let right = left + 1f32;
    let top = y.floor();
    let bottom = top + 1f32;

    let right_weight = x - left;
    let bottom_weight = y - top;

    if left < 0f32 || right >= texture_width as f32 || top < 0f32 || bottom >= texture_height as f32 {
        return default;
    }

    let indices_per_pixel = 4;
    let indices_per_pixel_usize = indices_per_pixel as usize;
    let left_u32 = left as u32;
    let top_u32 = top as u32;
    let right_u32 = right as u32;
    let bottom_u32 = bottom as u32;

    let top_left_red_index = get_red_index!(left_u32, top_u32, texture_width, indices_per_pixel);
    let top_right_red_index = get_red_index!(right_u32, top_u32, texture_width, indices_per_pixel);
    let bottom_left_red_index = get_red_index!(left_u32, bottom_u32, texture_width, indices_per_pixel);
    let bottom_right_red_index = get_red_index!(right_u32, bottom_u32, texture_width, indices_per_pixel);

    let top_left_red_index = top_left_red_index as usize;
    let top_right_red_index = top_right_red_index as usize;
    let bottom_left_red_index = bottom_left_red_index as usize;
    let bottom_right_red_index = bottom_right_red_index as usize;

    let top_left = &texture[top_left_red_index..top_left_red_index+indices_per_pixel_usize];
    let top_right = &texture[top_right_red_index..top_right_red_index+indices_per_pixel_usize];
    let bottom_left = &texture[bottom_left_red_index..bottom_left_red_index+indices_per_pixel_usize];
    let bottom_right = &texture[bottom_right_red_index..bottom_right_red_index+indices_per_pixel_usize];

    blend_bilinear(top_left, top_right, bottom_left, bottom_right, right_weight, bottom_weight)
}

pub fn rotate_texture_about_center(
    texture: &[u8],
    texture_width: u32,
    texture_height: u32,
    angle: f32,
    default_pixel: RgbaPixel,
) -> (Vec<u8>, u32, u32) {
    let angle = angle - (angle / 360.0).floor() * 360.0;
    let (dest_width, dest_height) = rotated_size(
        texture_width as usize, texture_height as usize, angle as f64
    );
    let num_pixels = (dest_height * dest_width) as usize;
    let indices_per_pixel = 4;
    let mut dest = vec![0; indices_per_pixel * num_pixels];

    let rotate = Matrix::rotate_degrees(angle);
    let (cx, cy) = (texture_width as f32 / 2.0, texture_height as f32 / 2.0);
    let rotate_about_center = Matrix::TranslateXY(cx, cy) * rotate * Matrix::TranslateXY(-cx, -cy);

    transform_texture(
        texture, texture_width, texture_height,
        &rotate_about_center, default_pixel,
        &mut dest, dest_width
    );

    (dest, dest_width, dest_height)
}


pub fn transform_texture(
    texture: &[u8],
    texture_width: u32,
    texture_height: u32,
    projection: &Matrix,
    default_pixel: RgbaPixel,
    out_texture: &mut Vec<u8>,
    out_width: u32,
) {
    let projection = projection.invert().unwrap();

    let indices_per_pixel = 4;
    let pitch = indices_per_pixel * out_width as usize;
    let chunks = out_texture.chunks_mut(pitch);

    chunks.enumerate().for_each(|(y, row)| {
        for (x, slice) in row.chunks_mut(indices_per_pixel).enumerate() {
            let (px, py) = projection.mul_point(x as f32, y as f32);
            let pixel = interpolate_bilinear(texture, texture_width, texture_height, px, py, default_pixel);
            slice[0] = pixel.r;
            slice[1] = pixel.g;
            slice[2] = pixel.b;
            slice[3] = pixel.a;
        }
    });
}


#[cfg(test)]
mod transform_tests {
    use super::*;
    use super::super::PIXEL_BLACK;

    #[test]
    fn blend_bilinear_works() {
        let top_left = [24, 24, 24, 0];
        let top_right = [22, 22, 22, 0];
        let bottom_left = [26, 26, 26, 0];
        let bottom_right = [26, 26, 26, 0];
        let right_weight = 0.37539673;
        let bottom_weight = 0.55303955;

        let blended_pixel = blend_bilinear(
            &top_left, &top_right, &bottom_left, &bottom_right,
            right_weight, bottom_weight
        );

        let expected_blended = RgbaPixel {
            r: 24, b: 24, g: 24, a: 255,
        };
        assert_eq!(blended_pixel, expected_blended);
    }

    #[test]
    fn interpolate_bilinear_works() {
        let top_left = [24, 24, 24, 0];
        let top_right = [22, 22, 22, 0];
        let bottom_left = [26, 26, 26, 0];
        let bottom_right = [26, 26, 26, 0];

        let texture = [
            top_left, top_right,
            bottom_left, bottom_right
        ].concat();

        let blended = interpolate_bilinear(
            &texture,
            2, 2, 0.37539673, 0.55303955,
            PIXEL_BLACK
        );
        let expected_blended = RgbaPixel {
            r: 24, b: 24, g: 24, a: 255,
        };
        assert_eq!(blended, expected_blended);
    }

    #[test]
    fn rotated_size_works() {
        // a 3x3 square:
        // [ a a a
        //   a a a
        //   a a a
        // ]
        // rotated 45 degrees should need to be in a 4x4:
        // [ x x a a
        //   x a a a
        //   a a a x
        //   x x x x
        //] ,,, something like that... its hard to do in ascii
        let (new_width, new_height) = rotated_size(3, 3, 45.0);
        assert_eq!(new_width, 4);
        assert_eq!(new_height, 4);
    }
}
