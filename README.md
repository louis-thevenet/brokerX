# Arc42 Documentation

See the [arc42](./docs/arc42/docs.md) file for detailed architecture documentation.

# Content

- API Gateway
- Microservices and their databases: user-service, order-service, market-data-service and order-processing-service
- Prometheus and Grafana setup
- Load tests with K6

# How to Run Locally

Create a `.env` file following indications from `.env.example`. (One will be provided in the archive, **it contains sensitive information**)

Start all containers: `docker compose up -d`

You can have finer control using Just. Run `just --list` for a list of available commands.

Information about accessing services will be printed in the terminal.

# Grafana

Grafana will be available at http://localhost:3000 (admin/admin123)

Then import the dashboard from `./grafana_dashboard.json`.

# Load tests

Enable MFA Bypass in `docker-compose.yml` by setting `MFA_BYPASS=true` in API Gateway section. Then `docker compose up -d` to restart the services.

Run `just load-test-setup` then `just load-test`.
