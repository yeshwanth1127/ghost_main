# Build stage - context is repo root
FROM rust:1-bookworm AS builder

WORKDIR /app

# Copy scribe-api
COPY scribe/scribe/scribe-api/Cargo.toml scribe/scribe/scribe-api/Cargo.lock* ./
COPY scribe/scribe/scribe-api/migrations ./migrations

# Create a dummy main to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release 2>/dev/null || true

# Copy actual source
COPY scribe/scribe/scribe-api/src ./src

# Build the real binary (reuse cached deps)
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/ghost-api /app/ghost-api
COPY --from=builder /app/migrations /app/migrations

ENV HOST=0.0.0.0
ENV PORT=8083

EXPOSE 8083

CMD ["./ghost-api"]
