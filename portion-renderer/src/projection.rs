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
    transform: [f32; 9],
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
    fn map_projective(&self, x: f32, y: f32) -> (f32, f32) {
        let t = &self.transform;
        let d = t[6] * x + t[7] * y + t[8];
        (
            (t[0] * x + t[1] * y + t[2]) / d,
            (t[3] * x + t[4] * y + t[5]) / d,
        )
    }

    #[inline(always)]
    fn map_affine(&self, x: f32, y: f32) -> (f32, f32) {
        let t = &self.transform;
        ((t[0] * x + t[1] * y + t[2]), (t[3] * x + t[4] * y + t[5]))
    }

    #[inline(always)]
    fn map_translation(&self, x: f32, y: f32) -> (f32, f32) {
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
