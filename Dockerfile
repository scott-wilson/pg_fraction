# ----------
# Build
# ----------
FROM postgres:14.1 as build-env

# Install build deps for Postgres and some Rust bindings.
RUN DEBIAN_FRONTEND="noninteractive" apt update && DEBIAN_FRONTEND="noninteractive" apt install -y \
    build-essential \
    clang \
    clang-11 \
    curl \
    curl \
    g++ \
    gdb \
    git \
    libclang-11-dev \
    libssl-dev \
    lldb-11 \
    make \
    pkg-config \
    postgresql-server-dev-14 \
    python3

# Install Rust
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:${PATH} \
    RUST_VERSION=1.58.1 \
    RUST_ARCH=x86_64-unknown-linux-gnu
RUN curl -O "https://static.rust-lang.org/rustup/archive/1.24.1/${RUST_ARCH}/rustup-init" && \
    chmod +x rustup-init && \
    ./rustup-init -y --no-modify-path --profile default --default-toolchain $RUST_VERSION --default-host ${RUST_ARCH} && \
    rm rustup-init && \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME && \
    rustup --version && \
    cargo --version && \
    rustc --version

WORKDIR /pg_fraction

# Switch to the postgres account because we can't run Postgres initdb as root.
USER postgres
# Insteall and set up cargo-pgx. We link to the version of postgres in the container, and don't try to build more than we have to.
# By default, cargo pgx init will download and build Postgres 10+ for testing.
RUN cargo install --version 0.2.6 cargo-pgx && \
    cargo pgx init --pg14 $(which pg_config)

# A trick to make the compile step cheaper by having the dependencies live on their own Docker layer.
USER root
COPY Cargo.toml .
RUN mkdir -p src && \
    echo 'fn main () {}' > src/lib.rs && \
    chown -R postgres /pg_fraction
USER postgres
RUN cargo build --release

# Compile and package pg_fraction
COPY . /pg_fraction
USER root
RUN cp -R /var/lib/postgresql/.pgx/ /root/.pgx/
RUN cargo pgx package

# ----------
# Deploy
# ----------
FROM postgres:14.1
COPY --from=build-env /pg_fraction/target/release/pg_fraction-pg14 /
COPY initdb.sql /docker-entrypoint-initdb.d/pg_fraction.sql

# At this point, doing a docker run should automatically activate the fraction type. You can test it with the following code:
# CREATE TABLE Test (number fraction NOT NULL);
# INSERT INTO Test(number) VALUES ('1/2');
# INSERT INTO Test(number) VALUES ('1');
# INSERT INTO Test(number) VALUES ('1/1');
# SELECT * FROM Test;

# You should see the following entries:
# - 1/2
# - 1
# - 1
