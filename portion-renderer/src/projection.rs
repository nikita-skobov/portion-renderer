use std::ops::Mul;

/// This module was a portion of
/// https://github.com/image-rs/imageproc/blob/master/src/geometric_transformations.rs
/// that was copy pasted here, and slightly modified
/// because I only need this part of it
/// Here is their license:

// The MIT License (MIT)

// Copyright (c) 2015 PistonDevelopers

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.



fn normalize(mx: [f32; 9]) -> [f32; 9] {
    [
        mx[0] / mx[8],
        mx[1] / mx[8],
        mx[2] / mx[8],
        mx[3] / mx[8],
        mx[4] / mx[8],
        mx[5] / mx[8],
        mx[6] / mx[8],
        mx[7] / mx[8],
        1.0,
    ]
}

// TODO: write me in f64
fn try_inverse(t: &[f32; 9]) -> Option<[f32; 9]> {
    let [t00, t01, t02, t10, t11, t12, t20, t21, t22] = t;

    let m00 = t11 * t22 - t12 * t21;
    let m01 = t10 * t22 - t12 * t20;
    let m02 = t10 * t21 - t11 * t20;

    let det = t00 * m00 - t01 * m01 + t02 * m02;

    if det.abs() < 1e-10 {
        return None;
    }

    let m10 = t01 * t22 - t02 * t21;
    let m11 = t00 * t22 - t02 * t20;
    let m12 = t00 * t21 - t01 * t20;
    let m20 = t01 * t12 - t02 * t11;
    let m21 = t00 * t12 - t02 * t10;
    let m22 = t00 * t11 - t01 * t10;

    #[rustfmt::skip]
    let inv = [
         m00 / det, -m10 / det,  m20 / det,
        -m01 / det,  m11 / det, -m21 / det,
         m02 / det, -m12 / det,  m22 / det,
    ];

    Some(normalize(inv))
}

fn mul3x3(a: [f32; 9], b: [f32; 9]) -> [f32; 9] {
    let [a00, a01, a02, a10, a11, a12, a20, a21, a22] = a;
    let [b00, b01, b02, b10, b11, b12, b20, b21, b22] = b;
    [
        a00 * b00 + a01 * b10 + a02 * b20,
        a00 * b01 + a01 * b11 + a02 * b21,
        a00 * b02 + a01 * b12 + a02 * b22,
        a10 * b00 + a11 * b10 + a12 * b20,
        a10 * b01 + a11 * b11 + a12 * b21,
        a10 * b02 + a11 * b12 + a12 * b22,
        a20 * b00 + a21 * b10 + a22 * b20,
        a20 * b01 + a21 * b11 + a22 * b21,
        a20 * b02 + a21 * b12 + a22 * b22,
    ]
}

// Classifies transformation by looking up transformation matrix coefficients
fn class_from_matrix(mx: [f32; 9]) -> TransformationClass {
    if (mx[6] - 0.0).abs() < 1e-10 && (mx[7] - 0.0).abs() < 1e-10 && (mx[8] - 1.0).abs() < 1e-10 {
        if (mx[0] - 1.0).abs() < 1e-10
            && (mx[1] - 0.0).abs() < 1e-10
            && (mx[3] - 0.0).abs() < 1e-10
            && (mx[4] - 1.0).abs() < 1e-10
        {
            TransformationClass::Translation
        } else {
            TransformationClass::Affine
        }
    } else {
        TransformationClass::Projection
    }
}


#[derive(Copy, Clone, Debug)]
enum TransformationClass {
    Translation,
    Affine,
    Projection,
}


/// A 2d projective transformation, stored as a row major 3x3 matrix.
///
/// Transformations combine by pre-multiplication, i.e. applying `P * Q` is equivalent to
/// applying `Q` and then applying `P`. For example, the following defines a rotation
/// about the point (320.0, 240.0).
///
/// ```
/// use imageproc::geometric_transformations::*;
/// use std::f32::consts::PI;
///
/// let (cx, cy) = (320.0, 240.0);
///
/// let c_rotation = Projection::translate(cx, cy)
///     * Projection::rotate(PI / 6.0)
///     * Projection::translate(-cx, -cy);
/// ```
///
/// See ./examples/projection.rs for more examples.
#[derive(Copy, Clone, Debug)]
pub struct Projection {
    pub transform: [f32; 9],
    inverse: [f32; 9],
    class: TransformationClass,
}

impl Projection {
    /// Creates a 2d projective transform from a row-major 3x3 matrix in homogeneous coordinates.
    ///
    /// Returns `None` if the matrix is not invertible.
    pub fn from_matrix(transform: [f32; 9]) -> Option<Projection> {
        let transform = normalize(transform);
        let class = class_from_matrix(transform);
        try_inverse(&transform).map(|inverse| Projection {
            transform,
            inverse,
            class,
        })
    }

    /// A translation by (tx, ty).
    #[rustfmt::skip]
    pub fn translate(tx: f32, ty: f32) -> Projection {
        Projection {
            transform: [
                1.0, 0.0, tx,
                0.0, 1.0, ty,
                0.0, 0.0, 1.0
            ],
            inverse: [
                1.0, 0.0, -tx,
                0.0, 1.0, -ty,
                0.0, 0.0, 1.0
            ],
            class: TransformationClass::Translation,
        }
    }

    /// A clockwise rotation around the top-left corner of the image by theta radians.
    #[rustfmt::skip]
    pub fn rotate(theta: f32) -> Projection {
        let (s, c) = theta.sin_cos();
        Projection {
            transform: [
                  c,  -s, 0.0,
                  s,   c, 0.0,
                0.0, 0.0, 1.0
            ],
            inverse: [
                  c,   s, 0.0,
                 -s,   c, 0.0,
                0.0, 0.0, 1.0
            ],
            class: TransformationClass::Affine,
        }
    }

    /// An anisotropic scaling (sx, sy).
    ///
    /// Note that the `warp` function does not change the size of the input image.
    /// If you want to resize an image then use the `imageops` module in the `image` crate.
    #[rustfmt::skip]
    pub fn scale(sx: f32, sy: f32) -> Projection {
        Projection {
            transform: [
                 sx, 0.0, 0.0,
                0.0,  sy, 0.0,
                0.0, 0.0, 1.0
            ],
            inverse: [
                1.0 / sx, 0.0,      0.0,
                0.0,      1.0 / sy, 0.0,
                0.0,      0.0,      1.0
            ],
            class: TransformationClass::Affine,
        }
    }

    /// Inverts the transformation.
    pub fn invert(self) -> Projection {
        Projection {
            transform: self.inverse,
            inverse: self.transform,
            class: self.class,
        }
    }

    // Helper functions used as optimization in warp.
    #[inline(always)]
    pub fn map_projective(&self, x: f32, y: f32) -> (f32, f32) {
        let t = &self.transform;
        let d = t[6] * x + t[7] * y + t[8];
        (
            (t[0] * x + t[1] * y + t[2]) / d,
            (t[3] * x + t[4] * y + t[5]) / d,
        )
    }

    #[inline(always)]
    pub fn map_affine(&self, x: f32, y: f32) -> (f32, f32) {
        let t = &self.transform;
        ((t[0] * x + t[1] * y + t[2]), (t[3] * x + t[4] * y + t[5]))
    }

    #[inline(always)]
    pub fn map_translation(&self, x: f32, y: f32) -> (f32, f32) {
        let t = &self.transform;
        let tx = t[2];
        let ty = t[5];
        (x + tx, y + ty)
    }
}

impl Mul<Projection> for Projection {
    type Output = Projection;

    fn mul(self, rhs: Projection) -> Projection {
        use TransformationClass as TC;
        let t = mul3x3(self.transform, rhs.transform);
        let i = mul3x3(rhs.inverse, self.inverse);

        let class = match (self.class, rhs.class) {
            (TC::Translation, TC::Translation) => TC::Translation,
            (TC::Translation, TC::Affine) => TC::Affine,
            (TC::Affine, TC::Translation) => TC::Affine,
            (TC::Affine, TC::Affine) => TC::Affine,
            (_, _) => TC::Projection,
        };

        Projection {
            transform: t,
            inverse: i,
            class,
        }
    }
}

impl<'a, 'b> Mul<&'b Projection> for &'a Projection {
    type Output = Projection;

    fn mul(self, rhs: &Projection) -> Projection {
        *self * *rhs
    }
}

impl Mul<(f32, f32)> for Projection {
    type Output = (f32, f32);

    fn mul(self, rhs: (f32, f32)) -> (f32, f32) {
        let (x, y) = rhs;
        match self.class {
            TransformationClass::Translation => self.map_translation(x, y),
            TransformationClass::Affine => self.map_affine(x, y),
            TransformationClass::Projection => self.map_projective(x, y),
        }
    }
}

impl<'a, 'b> Mul<&'b (f32, f32)> for &'a Projection {
    type Output = (f32, f32);

    fn mul(self, rhs: &(f32, f32)) -> (f32, f32) {
        *self * *rhs
    }
}


/// given theta in radians, convert to a sin, cos
/// rotation matrix
pub fn get_rotation_matrix_radians(theta: f32) -> Matrix {
    let (sin, cos) = theta.sin_cos();
    Matrix::Rotate(cos, sin)
}

/// given an angle in degrees, convert to a sin, cos
/// rotation matrix, this is just a convenience
/// function for calling get_rotation_matrix_radians
pub fn get_rotation_matrix(angle: f32) -> Matrix {
    let radians = angle.to_radians();
    get_rotation_matrix_radians(radians)
}

pub enum Matrix {
    Unit,
    Scale(f32, f32),
    TranslateXY(f32, f32),
    /// cos, sin
    Rotate(f32, f32),
    /// scalex, scaley, translatex, translatey
    ScaleAndTranslate(f32, f32, f32, f32),
    /// cos, sin, translatex, translatey
    RotateAndTranslate(f32, f32, f32, f32),
    /// 0, 1, 3, 4, translatex, translatey
    RotateAndScaleAndTranslate(f32, f32, f32, f32, f32, f32),
}

impl Mul<&(f32, f32)> for &Matrix {
    type Output = (f32, f32);

    #[inline(always)]
    fn mul(self, rhs: &(f32, f32)) -> Self::Output {
        match self {
            Matrix::Unit => *rhs,
            Matrix::Scale(sx, sy) => (sx * rhs.0, sy * rhs.1),
            Matrix::Rotate(cos, sin) => (cos * rhs.0 - sin * rhs.1, sin * rhs.0 + cos * rhs.1),
            Matrix::TranslateXY(by_x, by_y) => (rhs.0 + by_x, rhs.1 + by_y),
            Matrix::ScaleAndTranslate(sx, sy, by_x, by_y) => (sx * rhs.0 + by_x, sy * rhs.1 + by_y),
            Matrix::RotateAndTranslate(cos, sin, by_x, by_y) => (cos * rhs.0 - sin * rhs.1 + by_x, sin * rhs.0 + cos * rhs.1 + by_y),
            Matrix::RotateAndScaleAndTranslate(a0, a1, b0, b1, by_x, by_y) => (a0 * rhs.0 + a1 * rhs.1 + by_x, b0 * rhs.0 + b1 * rhs.1 + by_y),
        }
    }
}

impl Mul<(f32, f32)> for Matrix {
    type Output = (f32, f32);

    #[inline(always)]
    fn mul(self, rhs: (f32, f32)) -> Self::Output {
        &self * &rhs
    }
}

impl Mul<(f32, f32)> for &Matrix {
    type Output = (f32, f32);

    #[inline(always)]
    fn mul(self, rhs: (f32, f32)) -> Self::Output {
        self * &rhs
    }
}

impl From<&Matrix> for [f32; 9] {
    fn from(m: &Matrix) -> Self {
        match m {
            Matrix::Unit => [
                1.0, 0.0, 0.0,
                0.0, 1.0, 0.0,
                0.0, 0.0, 1.0],
            Matrix::Scale(sx, sy) => [
                *sx, 0.0, 0.0,
                0.0, *sy, 0.0,
                0.0, 0.0, 1.0],
            Matrix::Rotate(cos, sin) => [
                *cos, -*sin, 0.0,
                *sin, *cos, 0.0,
                0.0, 0.0, 1.0],
            Matrix::TranslateXY(x, y) => [
                1.0, 0.0, *x,
                0.0, 1.0, *y,
                0.0, 0.0, 1.0],
            Matrix::ScaleAndTranslate(sx, sy, by_x, by_y) => [
                *sx, 0.0, *by_x,
                0.0, *sy, *by_y,
                0.0, 0.0, 1.0],
            Matrix::RotateAndTranslate(cos, sin, by_x, by_y) => [
                *cos, -*sin, *by_x,
                *sin, *cos, *by_y,
                0.0, 0.0, 1.0],
            Matrix::RotateAndScaleAndTranslate(a0, a1, b0, b1, by_x, by_y) => [
                *a0, *a1, *by_x,
                *b0, *b1, *by_y,
                0.0, 0.0, 1.0],
        }
    }
}

impl From<Matrix> for [f32; 9] {
    fn from(m: Matrix) -> Self {
        (&m).into()
    }
}

pub fn print_matrix(m: &Matrix) {
    let matrix: [f32; 9] = m.into();
    print_matrix3(matrix);
}
pub fn print_matrix3(matrix: [f32; 9]) {
    for row in matrix.chunks(3) {
        for j in row {
            print!("{}, ", j);
        }
        println!("");
    }
}

impl Mul<Matrix> for Matrix {
    type Output = Matrix;

    fn mul(self, rhs: Matrix) -> Self::Output {
        let matrix_self: [f32; 9] = self.into();
        let matrix_rhs: [f32; 9] = rhs.into();

        let multiplied = mul3x3(matrix_self, matrix_rhs);

        let has_scale = match (multiplied[0], multiplied[4]) {
            (x, y) => if x == 1.0 && y == 1.0 {
                None
            } else {
                Some((x, y))
            }
        };
        let has_translate = match (multiplied[2], multiplied[5]) {
            (x, y) => if x == 0.0 && y == 0.0 {
                None
            } else {
                Some((x, y))
            }
        };
        let has_rotate = match (multiplied[1], multiplied[3]) {
            (x, y) => if x == 0.0 && y == 0.0 {
                None
            } else {
                Some((multiplied[0], multiplied[1], multiplied[3], multiplied[4]))
            }
        };

        let is_sin_and_cos = multiplied[0] == multiplied[4] && multiplied[1] == -multiplied[3];
        // check if its just rotate:
        if is_sin_and_cos && has_translate.is_none() {
            return Matrix::Rotate(multiplied[0], multiplied[3]);
        } else if is_sin_and_cos && has_translate.is_some() {
            let (tx, ty) = has_translate.unwrap(); // safe because is_some
            return Matrix::RotateAndTranslate(multiplied[0], multiplied[3], tx, ty);
        };

        match (has_scale, has_translate, has_rotate) {
            (None, None, None) => Matrix::Unit,
            (None, Some(translate), None) => Matrix::TranslateXY(translate.0, translate.1),
            (Some(scale), None, None) => Matrix::Scale(scale.0, scale.1),
            (Some(scale), Some(translate), None) => Matrix::ScaleAndTranslate(scale.0, scale.1, translate.0, translate.1),

            // I dont see a point in handling scale/rotate combinations
            // because their matrix positions overlap
            (_, None, Some(r)) => Matrix::RotateAndScaleAndTranslate(r.0, r.1, r.2, r.3, 0.0, 0.0),
            (_, Some(t), Some(r)) => Matrix::RotateAndScaleAndTranslate(r.0, r.1, r.2, r.3, t.0, t.1),
        }
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;

    #[inline(always)]
    fn assert_f_eq(float_left: f32, float_right: f32) {
        let threshhold = 10000.0;
        let left = f32::trunc(float_left * threshhold) / threshhold;
        let right = f32::trunc(float_right * threshhold) / threshhold;
        assert_eq!(left, right);
    }

    #[test]
    fn basic_matrix_multiplication_works() {
        let m_scale = Matrix::Scale(2.0, 3.0);
        let m_trans = Matrix::TranslateXY(1.0, 1.5);
        // matrix multiplication 'goes backwards'
        // this will first scale, then it will translate
        let m = m_trans * m_scale;
        if let Matrix::ScaleAndTranslate(_, _, _, _) = m {
            assert!(true);
        } else {
            print_matrix(&m);
            panic!("Matrix is not a scale and translate matrix");
        }
        let matrix: [f32; 9] = m.into();
        assert_eq!(matrix,
        [
            2.0, 0.0, 1.0,
            0.0, 3.0, 1.5,
            0.0, 0.0, 1.0,
        ]);

        // lets do the above in reverse order:
        let m_scale = Matrix::Scale(2.0, 3.0);
        let m_trans = Matrix::TranslateXY(1.0, 1.5);
        let m = m_scale * m_trans;
        let matrix: [f32; 9] = m.into();
        assert_eq!(matrix,
        [
            2.0, 0.0, 2.0,
            0.0, 3.0, 4.5,
            0.0, 0.0, 1.0,
        ]);
    }

    #[test]
    fn simple_rotate_works() {
        let (x, y) = (1.0, 0.0);
        let p = Projection::rotate(90f32.to_radians());
        let (out_x, out_y) = p.map_projective(x, y);
        assert_f_eq(out_x, 0.0);
        assert_f_eq(out_y, 1.0);

        let p = Projection::rotate((-90f32).to_radians());
        let (out_x, out_y) = p.map_projective(x, y);
        assert_f_eq(out_x, 0.0);
        assert_f_eq(out_y, -1.0);

        let p = Projection::rotate(45f32.to_radians());
        let (out_x, out_y) = p.map_projective(x, y);
        assert_f_eq(out_x, 0.70712);
        assert_f_eq(out_y, 0.70712);

        // going over 360 loops back
        // so p1 should have same result as p2
        let p1 = Projection::rotate(361f32.to_radians());
        let p2 = Projection::rotate(1f32.to_radians());
        let (p1_x, p1_y) = p1.map_projective(x, y);
        let (p2_x, p2_y) = p2.map_projective(x, y);
        assert_f_eq(p1_x, p2_x);
        assert_f_eq(p1_y, p2_y);
    }

    #[test]
    fn can_scale() {
        let (x, y) = (1.0, 0.0);
        let p = Projection::scale(2.0, 1.0);
        let (out_x, out_y) = p.map_projective(x, y);
        assert_f_eq(out_x, 2.0);
        assert_f_eq(out_y, 0.0);

        let m = Matrix::Scale(2.0, 1.0);
        let (out_x, out_y) = m * (x, y);
        assert_f_eq(out_x, 2.0);
        assert_f_eq(out_y, 0.0);
    }

    #[test]
    fn can_translate() {
        let (x, y) = (1.0, 0.0);
        let p = Projection::translate(1.0, 1.0);
        let (out_x, out_y) = p.map_projective(x, y);
        assert_f_eq(out_x, 2.0);
        assert_f_eq(out_y, 1.0);

        let m1 = Matrix::TranslateXY(1.0, 0.0);
        let m2 = Matrix::TranslateXY(0.0, 1.0);
        let m3 = m1 * m2;
        let (out_x, out_y) = m3 * (x, y);
        assert_f_eq(out_x, 2.0);
        assert_f_eq(out_y, 1.0);

        let m = Matrix::TranslateXY(1.0, 1.0);
        let (out_x, out_y) = m * (x, y);
        assert_f_eq(out_x, 2.0);
        assert_f_eq(out_y, 1.0);
    }

    #[test]
    fn can_rotate_and_scale() {
        let (x, y) = (1.0, 0.0);
        let p = Projection::rotate(90f32.to_radians()) * Projection::scale(2.0, 1.0);
        let (out_x, out_y) = p.map_projective(x, y);
        assert_f_eq(out_x, 0.0);
        assert_f_eq(out_y, 2.0);

        let rotation_matrix = get_rotation_matrix(90f32);
        let scale_matrix = Matrix::Scale(2.0, 1.0);
        let m = rotation_matrix * scale_matrix;
        let (out_x, out_y) = m * (x, y);
        assert_f_eq(out_x, 0.0);
        assert_f_eq(out_y, 2.0);
    }

    #[test]
    fn can_rotate_about_arbitrary_point() {
        let (x, y) = (1.0, 0.0);
        // normally a rotation 90d would move (1, 0) to (0, 1)
        // but here we rotate about an arbitrary point: (1, 1)
        // such that when our (1, 0) gets rotate 90d, it should become (2, 1)
        let p = Projection::translate(1.0, 1.0) * Projection::rotate(90f32.to_radians()) * Projection::translate(-1.0, -1.0);
        let (out_x, out_y) = p.map_projective(x, y);
        assert_f_eq(out_x, 2.0);
        assert_f_eq(out_y, 1.0);

        let r = get_rotation_matrix(90f32);
        let t1 = Matrix::TranslateXY(1.0, 1.0);
        let t2 = Matrix::TranslateXY(-1.0, -1.0);
        let m = t1 * r * t2;
        let (out_x, out_y) = m * (x, y);
        assert_f_eq(out_x, 2.0);
        assert_f_eq(out_y, 1.0);
    }
}
