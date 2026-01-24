use futures::stream::{self, StreamExt};
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[tokio::test]
async fn test_benchmark_concurrency() {
    let job_count = 50;
    let job_duration = Duration::from_millis(10);
    let concurrency = 10;

    println!("Benchmarking with {} jobs, each taking {:?}...", job_count, job_duration);

    // Sequential Benchmark
    let start_seq = Instant::now();
    for i in 0..job_count {
        process_job_simulated(i, job_duration).await;
    }
    let duration_seq = start_seq.elapsed();
    println!("Sequential execution time: {:?}", duration_seq);

    // Concurrent Benchmark
    let start_conc = Instant::now();
    stream::iter(0..job_count)
        .for_each_concurrent(Some(concurrency), |i| async move {
            process_job_simulated(i, job_duration).await;
        })
        .await;
    let duration_conc = start_conc.elapsed();
    println!("Concurrent execution time (limit={}): {:?}", concurrency, duration_conc);

    // Assert improvement
    if duration_seq.as_millis() > 0 {
        let speedup = duration_seq.as_secs_f64() / duration_conc.as_secs_f64();
        println!("Speedup: {:.2}x", speedup);
        assert!(speedup > 2.0, "Concurrent execution should be significantly faster");
    }
}

async fn process_job_simulated(_id: usize, duration: Duration) {
    sleep(duration).await;
}
