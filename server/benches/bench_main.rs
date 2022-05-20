use criterion::criterion_main;

mod benchmarks;

criterion_main! {
   benchmarks::router::benches,
   benchmarks::connections::benches,
}
