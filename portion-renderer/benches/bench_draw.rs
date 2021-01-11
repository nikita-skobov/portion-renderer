use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::{criterion_group, criterion_main};

use std::time::Duration;
use rand::prelude::*;

use portion_renderer;
use portion_renderer::bounds::Rect;
use portion_renderer::PortionRenderer;
use portion_renderer::PixelFormatEnum;
use portion_renderer::PIXEL_RED;

fn from_elem(c: &mut Criterion) {
    let mut rng = rand::thread_rng();

    let bounds = Rect {
        x: 0, y: 0,
        w: 1000,
        h: 1000,
    };

    let indices_per_pixel = 4;
    let max_pixel_index = (bounds.w * bounds.h * indices_per_pixel) as usize;
    let mut pixels: Vec<u8> = vec![0; max_pixel_index];
    let mut pixel_index = 0;
    while pixel_index < max_pixel_index {
        pixels[pixel_index] = rng.gen();
        pixels[pixel_index + 1] = rng.gen();
        pixels[pixel_index + 2] = rng.gen();
        pixels[pixel_index + 3] = 255; // alpha 100%
        pixel_index += 4;
    }
    let data = (bounds, pixels);


    let mut group = c.benchmark_group("draw");
    group.measurement_time(Duration::from_secs(8));
    group.bench_with_input(BenchmarkId::new("point_in_normal_rect", "data_vec"), &data, |b, s| {
        let (bounds, pixels) = s;
        let mut p = PortionRenderer::new_ex(1000, 1000, 10, 10, PixelFormatEnum::RGBA8888);
        b.iter(|| {
            p.draw(&pixels, *bounds);
        })
    });
    group.bench_with_input(BenchmarkId::new("draw_tilted_rect", "data_vec"), &"", |b, _| {
        let mut p = PortionRenderer::<u8>::new_ex(
            1000, 1000, 10, 10, PixelFormatEnum::RGBA8888
        );
        let red = p.create_object_from_color(
            1, Rect { x: 0, y: 0, w: 500, h: 400 },
            PIXEL_RED,
        );
        p.set_object_rotation(red, 45f32);
        p.move_object_x_by(red, 200);
        b.iter(|| {
            p.force_draw_all_layers();
        });
    });
}

criterion_group!(benches, from_elem);
criterion_main!(benches);
