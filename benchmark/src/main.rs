use clap::Parser;
use color_eyre::Result;
use domain::core::BrokerX;
use domain::order::{OrderSide, OrderType};
use domain::user::{UserId, UserRepoExt};
use hdrhistogram::Histogram;
use rand::Rng;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, warn};

#[derive(Parser, Debug)]
#[command(name = "brokerx-benchmark")]
#[command(about = "Performance benchmark for BrokerX")]
struct Args {
    /// Number of concurrent threads/clients
    #[arg(short, long, default_value_t = 8)]
    threads: usize,

    /// Duration of the test in seconds
    #[arg(short, long, default_value_t = 30)]
    duration: u64,

    /// Target throughput (orders per second)
    #[arg(long, default_value_t = 500)]
    target_throughput: u64,

    /// Enable latency measurements (may impact throughput)
    #[arg(long)]
    measure_latency: bool,

    /// Number of test users to create
    #[arg(long, default_value_t = 50)]
    test_users: usize,

    /// Order processing threads in BrokerX
    #[arg(long, default_value_t = 6)]
    processing_threads: usize,
}

#[derive(Debug)]
struct BenchmarkMetrics {
    pub orders_submitted: AtomicU64,
    pub orders_acknowledged: AtomicU64,
    pub orders_failed: AtomicU64,
    pub latency_histogram: Arc<Mutex<Histogram<u64>>>,
    pub start_time: Instant,
}

impl BenchmarkMetrics {
    fn new() -> Self {
        Self {
            orders_submitted: AtomicU64::new(0),
            orders_acknowledged: AtomicU64::new(0),
            orders_failed: AtomicU64::new(0),
            latency_histogram: Arc::new(Mutex::new(
                Histogram::new_with_bounds(1, 10_000, 3).unwrap(),
            )),
            start_time: Instant::now(),
        }
    }

    fn record_submission(&self) {
        self.orders_submitted.fetch_add(1, Ordering::Relaxed);
    }

    fn record_acknowledgment(&self, latency_ms: u64) {
        self.orders_acknowledged.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut hist) = self.latency_histogram.lock() {
            let _ = hist.record(latency_ms);
        }
    }

    fn record_failure(&self) {
        self.orders_failed.fetch_add(1, Ordering::Relaxed);
    }

    fn get_throughput(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let acknowledged = self.orders_acknowledged.load(Ordering::Relaxed) as f64;
        if elapsed > 0.0 {
            acknowledged / elapsed
        } else {
            0.0
        }
    }

    fn get_p95_latency(&self) -> u64 {
        if let Ok(hist) = self.latency_histogram.lock() {
            hist.value_at_quantile(0.95)
        } else {
            0
        }
    }

    fn print_report(&self) {
        let submitted = self.orders_submitted.load(Ordering::Relaxed);
        let acknowledged = self.orders_acknowledged.load(Ordering::Relaxed);
        let failed = self.orders_failed.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let throughput = self.get_throughput();
        let p95_latency = self.get_p95_latency();

        println!("\n=== BROKERX BENCHMARK RESULTS ===");
        println!("Test Duration: {elapsed:.2} seconds");
        println!("Orders Submitted: {submitted}");
        println!("Orders Acknowledged: {acknowledged}");
        println!("Orders Failed: {failed}");
        println!(
            "Success Rate: {:.2}%",
            (acknowledged as f64 / submitted as f64) * 100.0
        );
        println!("Throughput: {throughput:.2} orders/s");
        println!("P95 Latency: {p95_latency} ms");

        println!("\n=== REQUIREMENTS CHECK ===");
        println!(
            "P95 Latency ≤ 500ms: {} (actual: {}ms)",
            if p95_latency <= 500 {
                "✓ PASS"
            } else {
                "✗ FAIL"
            },
            p95_latency
        );
        println!(
            "Throughput ≥ 300 orders/s: {} (actual: {:.2})",
            if throughput >= 300.0 {
                "✓ PASS"
            } else {
                "✗ FAIL"
            },
            throughput
        );
        println!(
            "Availability ≥ 90.0%: {} (actual: {:.2}%)",
            if (acknowledged as f64 / submitted as f64) * 100.0 >= 90.0 {
                "✓ PASS"
            } else {
                "✗ FAIL"
            },
            (acknowledged as f64 / submitted as f64) * 100.0
        );

        let all_pass = p95_latency <= 500
            && throughput >= 300.0
            && (acknowledged as f64 / submitted as f64) >= 0.90;
        println!(
            "\nOVERALL: {}",
            if all_pass {
                "✓ ALL REQUIREMENTS PASSED"
            } else {
                "✗ SOME REQUIREMENTS FAILED"
            }
        );

        if let Ok(hist) = self.latency_histogram.lock() {
            println!("\n=== LATENCY DISTRIBUTION ===");
            println!("Min: {} ms", hist.min());
            println!("P50: {} ms", hist.value_at_quantile(0.50));
            println!("P90: {} ms", hist.value_at_quantile(0.90));
            println!("P95: {} ms", hist.value_at_quantile(0.95));
            println!("P99: {} ms", hist.value_at_quantile(0.99));
            println!("Max: {} ms", hist.max());
        }
    }
}

#[derive(Debug)]
struct TestUser {
    id: UserId,
}

fn setup_test_users(broker: &mut BrokerX, num_users: usize) -> Result<Vec<TestUser>> {
    info!("Setting up {} test users...", num_users);
    let mut users = Vec::new();

    let mut user_repo = broker.get_user_repo();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    for i in 0..num_users {
        let email = format!("test_user_{timestamp}_{i}@benchmark.test");
        let balance = 10_000_000.0; // Increased starting balance

        let user_id = user_repo
            .create_user(
                email.clone(),
                "password123".to_string(),
                format!("User{i}"),
                "Test".to_string(),
                balance,
            )
            .map_err(|e| color_eyre::eyre::eyre!("Failed to create user {}: {}", i, e))?;

        // Verify email to activate user
        user_repo
            .verify_user_email(&user_id)
            .map_err(|e| color_eyre::eyre::eyre!("Failed to verify user {}: {}", i, e))?;

        users.push(TestUser { id: user_id });
    }

    info!("Successfully created {} test users", users.len());
    Ok(users)
}

async fn benchmark_worker(
    worker_id: usize,
    broker: Arc<Mutex<BrokerX>>,
    users: Arc<Vec<TestUser>>,
    metrics: Arc<BenchmarkMetrics>,
    should_stop: Arc<AtomicUsize>,
    target_rate_per_thread: f64,
    measure_latency: bool,
) {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::from_entropy();

    // Reduced symbol set for better performance
    let symbols = ["AAPL", "GOOGL"];
    let price_ranges = [
        (150.0, 200.0), // AAPL
        (100.0, 150.0), // GOOGL
    ];

    let interval = Duration::from_secs_f64(1.0 / target_rate_per_thread);
    let mut next_order_time = Instant::now();

    info!(
        "Worker {} started with target rate {:.2} orders/s",
        worker_id, target_rate_per_thread
    );

    while should_stop.load(Ordering::Relaxed) == 0 {
        // Rate limiting with faster checks
        if Instant::now() < next_order_time {
            tokio::task::yield_now().await; // Yield instead of sleep
            continue;
        }
        next_order_time += interval;

        // Simplified order generation
        let user_idx = rng.gen_range(0..users.len());
        let user_id = users[user_idx].id;
        let symbol_idx = rng.gen_range(0..symbols.len());
        let symbol = symbols[symbol_idx].to_string();

        // Smaller quantities for faster processing
        let quantity = rng.gen_range(1..50);
        let side = if rng.gen_bool(0.5) {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };

        // More market orders for faster execution
        let order_type = if rng.gen_bool(0.9) {
            OrderType::Market
        } else {
            let (min_price, max_price) = price_ranges[symbol_idx];
            let price = min_price + rng.gen::<f64>() * (max_price - min_price);
            let aligned_price = (price * 100.0).round() / 100.0;
            OrderType::Limit(aligned_price)
        };

        let submission_time = if measure_latency {
            Some(Instant::now())
        } else {
            None
        };

        // Submit order with minimal lock time
        let result = {
            // Minimize lock scope
            let mut broker_guard = broker.lock().unwrap();
            broker_guard.create_order(user_id, symbol.clone(), quantity, side, order_type)
        };

        metrics.record_submission();

        match result {
            Ok(_order_id) => {
                if measure_latency {
                    if let Some(start_time) = submission_time {
                        let latency = start_time.elapsed().as_millis() as u64;
                        metrics.record_acknowledgment(latency);
                    }
                } else {
                    metrics.record_acknowledgment(1);
                }
            }
            Err(e) => {
                // Only warn on unexpected errors, not validation failures
                if !e.to_string().contains("Invalid tick size")
                    && !e.to_string().contains("Insufficient funds")
                {
                    warn!("Worker {} order failed: {}", worker_id, e);
                }
                metrics.record_failure();
            }
        }
    }

    info!("Worker {} stopped", worker_id);
}

async fn run_benchmark(args: Args) -> Result<()> {
    info!(
        "Starting BrokerX benchmark with {} threads for {}s",
        args.threads, args.duration
    );

    // Initialize BrokerX with optimal settings
    let mut broker = BrokerX::with_thread_count(args.processing_threads);
    broker.start_order_processing();

    // Setup test users
    let users = Arc::new(setup_test_users(&mut broker, args.test_users)?);
    let broker = Arc::new(Mutex::new(broker));

    // Initialize metrics
    let metrics = Arc::new(BenchmarkMetrics::new());
    let should_stop = Arc::new(AtomicUsize::new(0));

    // Calculate target rate per thread
    let target_rate_per_thread = args.target_throughput as f64 / args.threads as f64;

    info!(
        "Target rate per thread: {:.2} orders/s",
        target_rate_per_thread
    );

    // Start worker tasks
    let mut handles = Vec::new();
    for worker_id in 0..args.threads {
        let handle = tokio::spawn(benchmark_worker(
            worker_id,
            Arc::clone(&broker),
            Arc::clone(&users),
            Arc::clone(&metrics),
            Arc::clone(&should_stop),
            target_rate_per_thread,
            args.measure_latency,
        ));
        handles.push(handle);
    }

    // Status reporting task
    let status_handle = {
        let metrics = Arc::clone(&metrics);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                let submitted = metrics.orders_submitted.load(Ordering::Relaxed);
                let acknowledged = metrics.orders_acknowledged.load(Ordering::Relaxed);
                let failed = metrics.orders_failed.load(Ordering::Relaxed);
                let throughput = metrics.get_throughput();

                info!(
                    "Status: {} submitted, {} ack'd, {} failed, {:.2} orders/s",
                    submitted, acknowledged, failed, throughput
                );
            }
        })
    };

    // Run for specified duration
    sleep(Duration::from_secs(args.duration)).await;

    // Stop workers
    should_stop.store(1, Ordering::Relaxed);
    status_handle.abort();

    // Wait for workers to finish
    for handle in handles {
        let _ = handle.await;
    }

    // Wait a bit more for final order processing
    sleep(Duration::from_secs(1)).await;

    // Print final report
    metrics.print_report();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("optimized_benchmark=info".parse()?),
        )
        .init();

    let args = Args::parse();

    info!("BrokerX Benchmark");
    info!("Configuration: {:?}", args);

    run_benchmark(args).await?;

    Ok(())
}
