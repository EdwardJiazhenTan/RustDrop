# Build stage
FROM rust:1.81-alpine AS builder

# Install dependencies needed for building
RUN apk add --no-cache musl-dev pkgconfig openssl-dev

# Create app directory
WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy src directory to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this will be cached)
RUN cargo build --release && rm -rf src

# Copy source code
COPY src ./src

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM alpine:3.18

# Install runtime dependencies
RUN apk add --no-cache ca-certificates wget

# Create a non-root user
RUN addgroup -g 1001 -S rustdrop && \
    adduser -S -D -H -u 1001 -h /app -s /sbin/nologin -G rustdrop rustdrop

# Set working directory
WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/rustdrop /usr/local/bin/rustdrop

# Create directories for files and config
RUN mkdir -p /app/files /app/config && \
    chown -R rustdrop:rustdrop /app

# Switch to non-root user
USER rustdrop

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/api/health || exit 1

# Set environment variables
ENV RUSTDROP_SERVER__HOST=0.0.0.0 \
    RUSTDROP_FILES__DIRECTORY=/app/files

# Run the application
CMD ["rustdrop"] 