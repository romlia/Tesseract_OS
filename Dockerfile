# syntax=docker/dockerfile:1
FROM ubuntu:26.04 AS builder

# Set noninteractive installation
ENV DEBIAN_FRONTEND=noninteractive

# Install core build dependencies and native drivers
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libdrm-dev \
    libgbm-dev \
    libegl-dev \
    vulkan-tools \
    libvulkan-dev \
    libasound2-dev \
    libssl-dev \
    libudev-dev \
    libclang-dev \
    libseat-dev \
    udev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy workspace
WORKDIR /app
COPY . .

# Build the OS
WORKDIR /app/Tesseract_OS
RUN cargo build --release

# The runtime image
FROM ubuntu:26.04

ENV DEBIAN_FRONTEND=noninteractive

# Install runtime driver libraries
RUN apt-get update && apt-get install -y \
    libdrm2 \
    libgbm1 \
    libegl1 \
    libvulkan1 \
    mesa-vulkan-drivers \
    libasound2t64 \
    libasound2-plugins \
    pulseaudio-utils \
    udev \
    libssl3 \
    libseat1 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the ALSA configuration to force PulseAudio routing
COPY Tesseract_OS/asound.conf /etc/asound.conf

# Copy the compiled binary
COPY --from=builder /app/Tesseract_OS/target/release/prismatic-os /usr/local/bin/prismatic-os

# Execute the OS directly on bare metal
CMD ["prismatic-os"]
