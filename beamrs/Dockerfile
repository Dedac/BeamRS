FROM rust:1.89-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN useradd --create-home --uid 10001 beamrs
COPY --from=builder /app/target/release/beamrs /usr/local/bin/beamrs
COPY --from=builder /app/static ./static
ENV HOST=0.0.0.0
ENV PORT=8080
EXPOSE 8080
USER beamrs
CMD ["/usr/local/bin/beamrs"]
