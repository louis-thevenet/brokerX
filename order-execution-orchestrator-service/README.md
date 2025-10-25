# Order Execution Orchestrator

This microservice receives new order requests and orchestrates the full order lifecycle asynchronously, communicating with user, order, and market data services. It exposes a REST API for order submission.

## Endpoints

- `POST /orders` — Submit a new order (returns 202 Accepted)

## Running

```sh
cargo run
```

## Notes
- No database is used; all state is managed via service calls and in-memory queues (to be implemented).
- Extend with additional endpoints and logic as needed.
