Build notes:

- This requires pgx/cargo-pgx https://github.com/zombodb/pgx
- If you're running in a Docker env, then the Dockerfile should be sufficient
- pgx supports PostgreSQL from version 10 - 14. I've removed anything that isn't 14 related for my tests.
