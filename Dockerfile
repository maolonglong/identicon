FROM clux/muslrust:1.67.1 AS chef
RUN cargo install cargo-chef
WORKDIR /work/identicon

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /work/identicon/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

FROM gcr.io/distroless/static:nonroot
WORKDIR /
COPY --from=builder --chown=nonroot:nonroot /work/identicon/target/x86_64-unknown-linux-musl/release/identicon-server /
EXPOSE 8080
CMD ["/identicon-server", "--addr", "0.0.0.0:8080", "--concurrency", "64"]
