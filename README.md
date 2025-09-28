# Setup

## Database

1. Create a `.env` file following indications from `.env.example`.

Run the following command to start a PostgreSQL container:
`sudo docker run -d --name brokerx_postgres --env-file .env -p 5432:5432 postgres:16`

To reset the database, run:

```
sudo docker stop brokerx_postgres
sudo docker rm brokerx_postgres
```
