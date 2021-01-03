use super::RgbaPixel;
use super::get_red_index;
use super::Projection;
use super::PIXEL_BLACK;


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

    let mut out = [0, 0, 0, 0];
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

#[cfg(test)]
mod tests {
    use super::*;

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
            r: 24, b: 24, g: 24, a: 0,
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
            r: 24, b: 24, g: 24, a: 0,
        };
        assert_eq!(blended, expected_blended);
    }
}