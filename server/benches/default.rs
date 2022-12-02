use criterion::criterion_main;

mod benchmarks;

criterion_main! {
   benchmarks::router::benches,
    benchmarks::session_list::benches,
   // benchmarks::connections::benches,
}
