# Dockerfile

# ---- Builder Stage ----
FROM rust:1.85 as builder

# Set working directory
WORKDIR /usr/src/fermi_notifier

# Copy source code
COPY . .

# Install dependencies and build release binary
# Use cargo-chef for potentially better layer caching (optional but good practice)
# RUN cargo install cargo-chef
# COPY . .
# RUN cargo chef prepare --recipe-path recipe.json
# RUN cargo chef cook --release --recipe-path recipe.json
# Build directly for simplicity here:
RUN cargo build --release

# ---- Runtime Stage ----
# Use a minimal base image
FROM debian:bookworm-slim

# Install necessary runtime libraries (ca-certificates for HTTPS)
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/fermi_notifier/target/release/fermi_notifier .

# Set environment variable for port (Cloud Run expects 8080 by default, but respects PORT)
ENV PORT=8080
EXPOSE 8080

# Command to run the application
CMD ["./fermi_notifier"]
