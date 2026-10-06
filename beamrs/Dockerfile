FROM rust:1.86-bookworm AS builder
WORKDIR /app
COPY beamrs ./beamrs
WORKDIR /app/beamrs
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/beamrs/target/release/beamrs /usr/local/bin/beamrs
ENV HOST=0.0.0.0
ENV PORT=8080
EXPOSE 8080
CMD ["/usr/local/bin/beamrs"]
