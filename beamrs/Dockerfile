FROM rust:1.99-bookworm AS builder
WORKDIR /app
RUN rustup target add wasm32-unknown-unknown \
    && cargo install cargo-leptos --version 0.3.11 --locked
COPY . .
RUN cargo leptos build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN useradd --create-home --uid 10001 beamrs
COPY --from=builder /app/target/release/beamrs /usr/local/bin/beamrs
COPY --from=builder /app/target/site ./site
ENV HOST=0.0.0.0
ENV PORT=8080
ENV LEPTOS_SITE_ADDR=0.0.0.0:8080
ENV LEPTOS_SITE_ROOT=/app/site
ENV LEPTOS_SITE_PKG_DIR=pkg
ENV LEPTOS_OUTPUT_NAME=beamrs
ENV LEPTOS_ENV=PROD
EXPOSE 8080
USER beamrs
CMD ["/usr/local/bin/beamrs"]
