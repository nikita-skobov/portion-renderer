use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::SamplingMode;
use criterion::{criterion_group, criterion_main};

use rand::prelude::*;

use portion_renderer;

use portion_renderer::match_matrix;
use portion_renderer::projection::ComputePoint;
use portion_renderer::projection::Matrix;

fn takes_compute(c: impl ComputePoint, points: &Vec<(f32, f32)>) -> Vec<(f32, f32)> {
    points.iter().map(|pt| c.compute_pt(pt.0, pt.1)).collect::<Vec<(f32, f32)>>()
}

fn from_elem(c: &mut Criterion) {
    let m = Matrix::RotateAndScaleAndTranslate(
        1.4, 2.2, 0.9, 1.111, 3.1, 2.2
    );
    // old projection system that I compared to.
    // matrix is faster :)
    // let p = Projection::from_matrix([
    //     1.4, 2.2, 3.1,
    //     0.9, 1.111, 2.2,
    //     0.0, 0.0, 1.0,
    // ]).unwrap();
    // let m_array: [f32; 9] = (&m).into();
    // let p_array = p.transform;
    // assert_eq!(m_array, p_array);

    let mut rng = rand::thread_rng();

    let test_n_points = 1_000_000;
    let points: Vec<(f32, f32)> = (0..test_n_points).into_iter().map(|_| rng.gen()).collect();
    assert_eq!(points.len(), test_n_points);
    let data = (m, points);

    let mut group = c.benchmark_group("matrix");
    group.sampling_mode(SamplingMode::Flat);
    group.bench_with_input(BenchmarkId::new("matrix_mult", "data_vec"), &data, |b, s| {
        let (_m, points) = s;

        b.iter(|| {
            let res = match_matrix!(_m, takes_compute, points);
            res

            // points.iter().map(|pt| _p.map_affine(pt.0, pt.1)).collect::<Vec<(f32, f32)>>()
        })
    });
}

criterion_group!(benches, from_elem);
criterion_main!(benches);
