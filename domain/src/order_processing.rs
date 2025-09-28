use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use database_adapter::db::{DbError, Repository};
use rand::random;
use tracing::{debug, error, info};

use crate::order::{Order, OrderId, OrderRepo, OrderSide, OrderStatus};
use crate::user::{UserRepo, UserRepoExt};

/// Shared state between main thread and order processing threads
#[derive(Debug)]
pub struct SharedState {
    pub order_repo: OrderRepo,
    pub user_repo: UserRepo,
    pub order_queue: VecDeque<OrderId>,
    pub is_running: bool,
}

/// Order processing thread pool
#[derive(Debug)]
pub struct ProcessingPool {
    worker_handles: Vec<thread::JoinHandle<()>>,
    pub shared_state: Arc<Mutex<SharedState>>,
    work_available: Arc<Condvar>,
    should_stop: Arc<Mutex<bool>>,
}
enum ProcessingError {
    DbError(DbError),
    OrderNotFound,
    CantCancel,
}
impl ProcessingPool {
    pub fn new(num_threads: usize) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            order_repo: OrderRepo::new("orders").expect("orders repo failed to load"),
            user_repo: UserRepo::new("users").expect("users repo failed to load"),
            order_queue: VecDeque::new(),
            is_running: false,
        }));

        // Get all queued and pending orders from the database and add them to the queue
        {
            let mut state = shared_state.lock().unwrap();
            match state.order_repo.find_all_by_field("status", "Pending") {
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

        let work_available = Arc::new(Condvar::new());
        let should_stop = Arc::new(Mutex::new(false));
        let mut worker_handles = Vec::new();

        // Spawn worker threads
        for thread_id in 0..num_threads {
            let shared_state_clone = Arc::clone(&shared_state);
            let work_available_clone = Arc::clone(&work_available);
            let should_stop_clone = Arc::clone(&should_stop);

            let handle = thread::spawn(move || {
                Self::worker_thread(
                    thread_id,
                    &shared_state_clone,
                    &work_available_clone,
                    &should_stop_clone,
                );
            });

            worker_handles.push(handle);
        }

        info!("Started order processing pool with {} threads", num_threads);

        Self {
            worker_handles,
            shared_state,
            work_available,
            should_stop,
        }
    }

    fn worker_thread(
        thread_id: usize,
        shared_state: &Arc<Mutex<SharedState>>,
        work_available: &Arc<Condvar>,
        should_stop: &Arc<Mutex<bool>>,
    ) {
        debug!("Order processing thread {} started", thread_id);

        loop {
            // Check if we should stop
            {
                let stop = should_stop.lock().unwrap();
                if *stop {
                    debug!("Order processing thread {} stopping", thread_id);
                    break;
                }
            }

            // Get next order to process
            let order_id = {
                let mut state = shared_state.lock().unwrap();

                // Wait for work if queue is empty
                while state.order_queue.is_empty() && state.is_running {
                    let stop = should_stop.lock().unwrap();
                    if *stop {
                        break;
                    }
                    drop(stop);

                    // Use wait_timeout with longer timeout to reduce CPU usage when idle
                    let (inner_state, _timeout_result) = work_available
                        .wait_timeout(state, Duration::from_millis(1000))
                        .unwrap();
                    state = inner_state;
                }

                // Check again if we should stop
                let stop = should_stop.lock().unwrap();
                if *stop {
                    break;
                }
                drop(stop);

                state.order_queue.pop_front()
            };

            // Process the order if we got one
            if let Some(order_id) = order_id {
                if let Ok(()) = Self::process_order(thread_id, order_id, shared_state) {
                } else {
                    error!("Thread {} failed to process order {}", thread_id, order_id);
                }

                // Add a small delay after processing to prevent tight loops
                // This is especially important for orders that get re-queued
                thread::sleep(Duration::from_millis(10));
            } else {
                // No work available, sleep longer to reduce CPU usage when idle
                thread::sleep(Duration::from_millis(100));
            }
        }

        debug!("Order processing thread {} terminated", thread_id);
    }

    fn process_order(
        thread_id: usize,
        order_id: OrderId,
        shared_state: &Arc<Mutex<SharedState>>,
    ) -> Result<(), ProcessingError> {
        let mut state = shared_state.lock().unwrap();

        if let Some(mut order) = state
            .order_repo
            .get(&order_id)
            .map_err(ProcessingError::DbError)?
        {
            let old_status = format!("{:?}", order.status);

            match &order.status {
                OrderStatus::Queued => {
                    debug!("Thread {} processing queued order {}", thread_id, order_id);
                    // Move to pending status
                    order.status = OrderStatus::Pending;
                    // Re-queue for further processing
                    state.order_queue.push_back(order_id);
                }
                OrderStatus::Pending => {
                    debug!("Thread {} executing pending order {}", thread_id, order_id);
                    // Simulate order matching with randomization
                    match random::<u32>() % 4 {
                        0 => {
                            let execution_price = 100.0;
                            if let Ok(()) = match order.order_side {
                                OrderSide::Buy => {
                                    // Deduct funds from user's account
                                    state.user_repo.withdraw_from_user(
                                        &order.client_id,
                                        execution_price * order.quantity as f64,
                                    )
                                }
                                OrderSide::Sell => {
                                    // Add funds to user's account
                                    state.user_repo.deposit_to_user(
                                        &order.client_id,
                                        execution_price * order.quantity as f64,
                                    )
                                }
                            } {
                                Self::update_portfolio_for_filled_order(
                                    &mut state,
                                    &order,
                                    execution_price,
                                );
                                // Order filled completely
                                order.status = OrderStatus::Filled {
                                    date: chrono::Utc::now().naive_local(),
                                };
                                info!("Thread {} filled order {} completely", thread_id, order_id);
                            } else {
                                // Failed to update user funds, reject order
                                order.status = OrderStatus::Rejected {
                                    date: chrono::Utc::now().naive_local(),
                                };
                                error!(
                                    "Thread {} rejected order {} due to insufficient funds",
                                    thread_id, order_id
                                );
                            }
                        }

                        // 2 => {
                        //     // Order rejected
                        //     order.status = OrderStatus::Rejected {
                        //         date: chrono::Utc::now().naive_local(),
                        //     };
                        //     info!("Thread {} rejected order {}", thread_id, order_id);
                        // }
                        _ => {
                            // Keep pending, re-queue
                            state.order_queue.push_back(order_id);
                        }
                    }
                }
                OrderStatus::PendingCancel => {
                    debug!("Thread {} cancelling order {}", thread_id, order_id);
                    order.status = OrderStatus::Cancelled;
                    info!("Thread {} cancelled order {}", thread_id, order_id);
                }
                _ => {
                    error!(
                        "Thread {} encountered order {} in unexpected state: {}",
                        thread_id, order_id, old_status
                    );
                }
            }
            state
                .order_repo
                .update(order_id, order)
                .map_err(ProcessingError::DbError)?;
        } else {
            error!(
                "Thread {} could not find order {} in repository",
                thread_id, order_id
            );
        }
        Ok(())
    }

    /// Submit a new order for processing
    pub fn submit_order(&self, order_id: OrderId) {
        let mut state = self.shared_state.lock().unwrap();
        state.order_queue.push_back(order_id);
        state.is_running = true;

        // Notify worker threads that work is available
        self.work_available.notify_one();

        debug!(
            "Submitted order {} to processing pool (queue size: {})",
            order_id,
            state.order_queue.len()
        );
    }

    /// Start processing orders
    pub fn start(&self) {
        let mut state = self.shared_state.lock().unwrap();
        state.is_running = true;
        info!("Order processing pool started");
    }

    /// Stop processing new orders and signal threads to terminate
    pub fn stop(&self) {
        {
            let mut state = self.shared_state.lock().unwrap();
            state.is_running = false;
        }

        {
            let mut stop = self.should_stop.lock().unwrap();
            *stop = true;
        }

        // Wake up all waiting threads
        self.work_available.notify_all();

        info!("Order processing pool stop signal sent");
    }

    pub fn shutdown(self) {
        info!("Shutting down order processing pool...");
        self.stop();

        // Wait for all threads to finish
        for (i, handle) in self.worker_handles.into_iter().enumerate() {
            if let Err(e) = handle.join() {
                error!("Failed to join worker thread {}: {:?}", i, e);
            }
        }

        info!("Order processing pool shutdown complete");
    }

    #[must_use]
    /// Get current size of the order queue
    pub fn get_queue_size(&self) -> usize {
        let state = self.shared_state.lock().unwrap();
        state.order_queue.len()
    }

    /// Get a copy of an order by its ID
    pub fn get_order(&self, order_id: &OrderId) -> Result<Option<Order>, ProcessingError> {
        let state = self.shared_state.lock().unwrap();
        state
            .order_repo
            .get(order_id)
            .map_err(ProcessingError::DbError)
    }

    /// Cancel an order (sets it to `PendingCancel` status)
    pub fn cancel_order(&self, order_id: &OrderId) -> Result<(), ProcessingError> {
        let mut state = self.shared_state.lock().unwrap();

        if let Some(mut order) = state
            .order_repo
            .get(order_id)
            .map_err(ProcessingError::DbError)?
        {
            match order.status {
                OrderStatus::Queued | OrderStatus::Pending => {
                    order.status = OrderStatus::PendingCancel;
                    // Re-queue for processing the cancellation
                    state.order_queue.push_back(*order_id);
                    state
                        .order_repo
                        .update(*order_id, order)
                        .map_err(ProcessingError::DbError)?;
                    Ok(())
                }
                _ => Err(ProcessingError::CantCancel),
            }
        } else {
            Err(ProcessingError::OrderNotFound)
        }
    }

    /// Update portfolio when an order is filled
    fn update_portfolio_for_filled_order(
        state: &mut SharedState,
        order: &Order,
        execution_price: f64,
    ) {
        let quantity_change = match order.order_side {
            OrderSide::Buy => order.quantity as i64,
            OrderSide::Sell => -(order.quantity as i64),
        };

        // Update the user's holdings
        match state.user_repo.get(&order.client_id) {
            Ok(Some(mut user)) => {
                user.update_holding(&order.symbol, quantity_change, execution_price);
                if let Err(e) = state.user_repo.update(order.client_id, user) {
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
