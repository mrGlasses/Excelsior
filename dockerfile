# ============================================
# Production-Ready Multi-Stage Dockerfile
# Optimized with sccache for fast compilation
# Based on original architecture + sccache caching
# ============================================

# ============================================
# Stage 1: Dependency Builder (Cached Layer)
# ============================================
FROM rustlang/rust:nightly-bookworm-slim AS builder

# Install build-time dependencies + sccache
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Install sccache for compilation caching
# Auto-detect architecture (works on both ARM64 and AMD64)
RUN ARCH=$(uname -m) && \
    if [ "$ARCH" = "aarch64" ]; then ARCH="aarch64"; fi && \
    if [ "$ARCH" = "x86_64" ]; then ARCH="x86_64"; fi && \
    wget -q https://github.com/mozilla/sccache/releases/download/v0.7.7/sccache-v0.7.7-${ARCH}-unknown-linux-musl.tar.gz && \
    tar xzf sccache-v0.7.7-${ARCH}-unknown-linux-musl.tar.gz && \
    mv sccache-v0.7.7-${ARCH}-unknown-linux-musl/sccache /usr/local/bin/ && \
    chmod +x /usr/local/bin/sccache && \
    rm -rf sccache-v0.7.7-${ARCH}-unknown-linux-musl*

# Configure sccache
ENV RUSTC_WRAPPER=/usr/local/bin/sccache
ENV SCCACHE_DIR=/sccache
ENV CARGO_INCREMENTAL=0

WORKDIR /app

# ============================================
# CRITICAL: Dependency Caching Strategy
# ============================================
# Copy only Cargo manifests first to create a cached dependency layer
# This layer will only rebuild if Cargo.toml or Cargo.lock changes
COPY Cargo.toml Cargo.lock ./

# Create a dummy source file to build dependencies
# This allows Docker to cache the compiled dependencies separately from your code
RUN mkdir -p src && \
    echo "fn main() {println!(\"Dummy for dependency caching\");}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# ============================================
# Stage 2: Application Builder
# ============================================
# Copy the actual source code
COPY . .

# Touch main.rs to ensure it's rebuilt with actual code
# This forces Cargo to recognize the file change
RUN touch src/main.rs

# Build the actual application binary with sccache and Docker layer caching
# Mount caches to persist between builds (requires BuildKit)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/sccache \
    cargo build --release --bin ms1

# Show sccache statistics (for debugging build performance)
RUN sccache --show-stats || true

# ============================================
# Stage 3: Runtime Image (Minimal & Secure)
# ============================================
FROM debian:bookworm-slim

# Install ONLY runtime dependencies (no build tools)
# This keeps the image small and secure
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# ============================================
# Security: Non-root User
# ============================================
# Create a dedicated user to run the application
# This follows the principle of least privilege
RUN useradd -m -u 1001 -s /bin/bash appuser

WORKDIR /app

# Copy the compiled binary from the builder stage
# Only the final binary is included, not source code or build artifacts
COPY --from=builder /app/target/release/ms1 /app/ms1

# Change ownership to the non-root user
RUN chown -R appuser:appuser /app

# Switch to the non-root user
USER appuser

# ============================================
# Runtime Configuration
# ============================================
# Expose the application port
# This should match MS_PORT in your .env file
EXPOSE 3000

# Health check endpoint
# Uses the /ping route defined in your Axum router
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3000/ping || exit 1

# ============================================
# Entrypoint
# ============================================
# Run the binary
ENTRYPOINT ["/app/ms1"]