use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use database_adapter::db::Repository;
use rand::random;
use tokio::sync::{Mutex, Notify};
use tokio::time::sleep;
use tracing::{debug, error, info};

use crate::order::{Order, OrderId, OrderRepo, OrderSide, OrderStatus};
use crate::user::{UserRepo, UserRepoExt};

/// Shared state between main task and order processing tasks
#[derive(Debug)]
pub struct SharedState {
    pub order_repo: OrderRepo,
    pub user_repo: UserRepo,
    pub order_queue: VecDeque<OrderId>,
    pub is_running: bool,
}

/// Order processing task pool
#[derive(Debug)]
pub struct ProcessingPool {
    _worker_handles: Vec<tokio::task::JoinHandle<()>>,
    pub shared_state: Arc<Mutex<SharedState>>,
    work_available: Arc<Notify>,
    should_stop: Arc<Mutex<bool>>,
}

#[derive(Debug)]
enum ProcessingError {
    DbError,
}
impl ProcessingPool {
    pub async fn new(num_threads: usize) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            order_repo: OrderRepo::new("orders")
                .await
                .expect("orders repo failed to load"),
            user_repo: UserRepo::new("users")
                .await
                .expect("users repo failed to load"),
            order_queue: VecDeque::new(),
            is_running: false,
        }));

        // Get all queued and pending orders from the database and add them to the queue
        {
            let mut state = shared_state.lock().await;
            match state
                .order_repo
                .find_all_by_field("status", "Pending")
                .await
            {
                Ok(orders) => {
                    for (uuid, _order) in orders {
                        state.order_queue.push_back(uuid);
                    }
                    info!(
                        "Loaded {} queued/pending orders into processing queue",
                        state.order_queue.len()
                    );
                }
                Err(e) => {
                    error!("Failed to load queued/pending orders: {}", e);
                }
            }
        }

        let work_available = Arc::new(Notify::new());
        let should_stop = Arc::new(Mutex::new(false));
        let mut worker_handles = Vec::new();

        // Spawn worker tasks
        for thread_id in 0..num_threads {
            let shared_state_clone = Arc::clone(&shared_state);
            let work_available_clone = Arc::clone(&work_available);
            let should_stop_clone = Arc::clone(&should_stop);

            let handle = tokio::spawn(async move {
                Self::worker_task(
                    thread_id,
                    shared_state_clone,
                    work_available_clone,
                    should_stop_clone,
                )
                .await;
            });

            worker_handles.push(handle);
        }

        info!("Started order processing pool with {} tasks", num_threads);

        Self {
            _worker_handles: worker_handles,
            shared_state,
            work_available,
            should_stop,
        }
    }
    /// Create `ProcessingPool` for testing with unique table names to avoid conflicts
    pub async fn new_for_testing(num_threads: usize) -> Self {
        use uuid::Uuid;
        let test_id = Uuid::new_v4().simple().to_string();
        let orders_table = format!("orders_test_{}", &test_id[..8]);
        let users_table = format!("users_test_{}", &test_id[..8]);

        let shared_state = Arc::new(Mutex::new(SharedState {
            order_repo: OrderRepo::new(&orders_table)
                .await
                .expect("orders repo failed to load"),
            user_repo: UserRepo::new(&users_table)
                .await
                .expect("users repo failed to load"),
            order_queue: VecDeque::new(),
            is_running: false,
        }));

        // Skip loading existing orders for tests to keep them isolated
        let work_available = Arc::new(Notify::new());
        let should_stop = Arc::new(Mutex::new(false));
        let mut worker_handles = Vec::new();

        // Spawn worker tasks
        for thread_id in 0..num_threads {
            let shared_state_clone = Arc::clone(&shared_state);
            let work_available_clone = Arc::clone(&work_available);
            let should_stop_clone = Arc::clone(&should_stop);

            let handle = tokio::spawn(async move {
                Self::worker_task(
                    thread_id,
                    shared_state_clone,
                    work_available_clone,
                    should_stop_clone,
                )
                .await;
            });

            worker_handles.push(handle);
        }

        info!(
            "Started test order processing pool with {} tasks",
            num_threads
        );

        Self {
            _worker_handles: worker_handles,
            shared_state,
            work_available,
            should_stop,
        }
    }

    async fn worker_task(
        thread_id: usize,
        shared_state: Arc<Mutex<SharedState>>,
        work_available: Arc<Notify>,
        should_stop: Arc<Mutex<bool>>,
    ) {
        debug!("Order processing task {} started", thread_id);

        loop {
            // Check if we should stop
            {
                let stop = should_stop.lock().await;
                if *stop {
                    debug!("Order processing task {} stopping", thread_id);
                    break;
                }
            }

            // Get next order to process
            let order_id = {
                let mut state = shared_state.lock().await;

                // Wait for work if queue is empty
                while state.order_queue.is_empty() && state.is_running {
                    let stop = should_stop.lock().await;
                    if *stop {
                        break;
                    }
                    drop(stop);
                    drop(state);

                    // Wait for notification or timeout
                    tokio::select! {
                        _ = work_available.notified() => {},
                        _ = sleep(Duration::from_millis(1000)) => {},
                    }

                    state = shared_state.lock().await;
                }

                // Check again if we should stop
                let stop = should_stop.lock().await;
                if *stop {
                    break;
                }
                drop(stop);

                state.order_queue.pop_front()
            };

            // Process the order if we got one
            if let Some(order_id) = order_id {
                if let Err(_) = Self::process_order(thread_id, order_id, &shared_state).await {
                    error!("Task {} failed to process order {}", thread_id, order_id);
                }

                // Add a small delay after processing to prevent tight loops
                // This is especially important for orders that get re-queued
                sleep(Duration::from_millis(10)).await;
            } else {
                // No work available, sleep longer to reduce CPU usage when idle
                sleep(Duration::from_millis(100)).await;
            }
        }

        debug!("Order processing task {} terminated", thread_id);
    }

    async fn process_order(
        thread_id: usize,
        order_id: OrderId,
        shared_state: &Arc<Mutex<SharedState>>,
    ) -> Result<(), ProcessingError> {
        let mut state = shared_state.lock().await;

        if let Some(mut order) = state
            .order_repo
            .get(&order_id)
            .await
            .map_err(|_e| ProcessingError::DbError)?
        {
            let old_status = format!("{:?}", order.status);

            match &order.status {
                OrderStatus::Queued => {
                    debug!("Task {} processing queued order {}", thread_id, order_id);
                    // Move to pending status
                    order.status = OrderStatus::Pending;
                    // Re-queue for further processing
                    state.order_queue.push_back(order_id);
                }
                OrderStatus::Pending => {
                    debug!("Task {} executing pending order {}", thread_id, order_id);
                    // Simulate order matching with randomization
                    match random::<u32>() % 4 {
                        0 => {
                            let execution_price = 100.0;

                            let funds_result = match order.order_side {
                                OrderSide::Buy => {
                                    // Deduct funds from user's account
                                    state
                                        .user_repo
                                        .withdraw_from_user(
                                            &order.client_id,
                                            execution_price * order.quantity as f64,
                                        )
                                        .await
                                }
                                OrderSide::Sell => {
                                    // Add funds to user's account
                                    state
                                        .user_repo
                                        .deposit_to_user(
                                            &order.client_id,
                                            execution_price * order.quantity as f64,
                                        )
                                        .await
                                }
                            };

                            if funds_result.is_ok() {
                                Self::update_portfolio_for_filled_order_async(
                                    &state,
                                    &order,
                                    execution_price,
                                )
                                .await;
                                // Order filled completely
                                order.status = OrderStatus::Filled {
                                    date: chrono::Utc::now().naive_local(),
                                };
                                info!("Task {} filled order {} completely", thread_id, order_id);
                            } else {
                                // Failed to update user funds, reject order
                                order.status = OrderStatus::Rejected {
                                    date: chrono::Utc::now().naive_local(),
                                };
                                error!(
                                    "Task {} rejected order {} due to insufficient funds",
                                    thread_id, order_id
                                );
                            }
                        }
                        _ => {
                            // Keep pending, re-queue
                            state.order_queue.push_back(order_id);
                        }
                    }
                }
                OrderStatus::PendingCancel => {
                    debug!("Task {} cancelling order {}", thread_id, order_id);
                    order.status = OrderStatus::Cancelled;
                    info!("Task {} cancelled order {}", thread_id, order_id);
                }
                _ => {
                    error!(
                        "Task {} encountered order {} in unexpected state: {}",
                        thread_id, order_id, old_status
                    );
                }
            }

            state
                .order_repo
                .update(order_id, order)
                .await
                .map_err(|_e| ProcessingError::DbError)?;
        } else {
            error!(
                "Task {} could not find order {} in repository",
                thread_id, order_id
            );
        }
        Ok(())
    }

    /// Submit a new order for processing
    pub async fn submit_order(&self, order_id: OrderId) {
        let mut state = self.shared_state.lock().await;
        state.order_queue.push_back(order_id);
        state.is_running = true;

        // Notify worker tasks that work is available
        self.work_available.notify_one();

        debug!(
            "Submitted order {} to processing pool (queue size: {})",
            order_id,
            state.order_queue.len()
        );
    }

    /// Start processing orders
    pub async fn start(&self) {
        let mut state = self.shared_state.lock().await;
        state.is_running = true;
        info!("Order processing pool started");
    }

    /// Stop processing new orders and signal tasks to terminate
    pub async fn stop(&self) {
        {
            let mut state = self.shared_state.lock().await;
            state.is_running = false;
        }

        {
            let mut stop = self.should_stop.lock().await;
            *stop = true;
        }

        // Wake up all waiting tasks
        self.work_available.notify_waiters();

        info!("Order processing pool stop signal sent");
    }

    /// Update portfolio when an order is filled
    async fn update_portfolio_for_filled_order_async(
        state: &SharedState,
        order: &Order,
        execution_price: f64,
    ) {
        let quantity_change = match order.order_side {
            OrderSide::Buy => order.quantity as i64,
            OrderSide::Sell => -(order.quantity as i64),
        };

        // Update the user's holdings
        match state.user_repo.get(&order.client_id).await {
            Ok(Some(mut user)) => {
                user.update_holding(&order.symbol, quantity_change, execution_price);
                if let Err(e) = state.user_repo.update(order.client_id, user).await {
                    error!(
                        "Failed to save updated user {} after order {}: {}",
                        order.client_id, order.symbol, e
                    );
                } else {
                    info!(
                        "Updated portfolio for user {}: {} {} shares of {} at ${}",
                        order.client_id,
                        if quantity_change > 0 {
                            "bought"
                        } else {
                            "sold"
                        },
                        quantity_change.abs(),
                        order.symbol,
                        execution_price
                    );
                }
            }
            Ok(None) => {
                error!(
                    "User {} not found when trying to update holdings after order {}",
                    order.client_id, order.symbol
                );
            }
            Err(e) => {
                error!(
                    "Failed to load user {} for portfolio update after order {}: {}",
                    order.client_id, order.symbol, e
                );
            }
        }
    }
}
