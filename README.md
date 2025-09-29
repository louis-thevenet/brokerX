# How to Run

## Database

1. Create a `.env` file following indications from `.env.example`.

Run the following command to start a PostgreSQL container:
`sudo docker run -d --name brokerx_postgres --env-file .env -p 5432:5432 postgres:16`

To reset the database, run:

```
sudo docker stop brokerx_postgres
sudo docker rm brokerx_postgres
```

## Application

To start the application, run: `cargo run --release --package app` then open [`localhost:5000`](http://localhost:5000) in your browser.

Test user:

- username: `test@test.com`
- password: `aaaaaa`
- OTP code is always `000000`

# Benchmark

To run the benchmark:

```bash
RUST_LOG=domain=debug cargo run -r --bin benchmark -- --threads 4 --processing-threads 4 --duration 15 --target-throughput 200 --test-users 5
```
