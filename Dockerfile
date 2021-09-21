FROM rust:1.55-slim-buster

WORKDIR /home/rust/

# Avoid having to install/build all dependencies by copying
# the Cargo files and making a dummy src/main.rs
COPY Cargo.toml .
COPY Cargo.lock .
RUN mkdir src
RUN echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN cargo test --release

# We need to touch our real main.rs file or else docker will use
# the cached one.
COPY . .
RUN touch src/main.rs

RUN cargo build --release
RUN cargo test --release

# Size optimization
RUN strip target/release/awair-prometheus-exporter

EXPOSE 8888
ENTRYPOINT ["/home/rust/target/release/awair-prometheus-exporter"]
