FROM rust:1.72 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12:latest-arm64
COPY --from=builder /app/target/release/bipper /
COPY --from=builder /app/.env /
CMD ["/bipper"]
