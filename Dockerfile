# Use official Postgres image as base
FROM postgres:16

# Place your SQL files in ./init/ and they will run on startup
# COPY init/ /docker-entrypoint-initdb.d/

