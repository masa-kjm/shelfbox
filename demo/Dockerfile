# syntax=docker/dockerfile:1

# Build the exact shelfbox revision being demonstrated.
FROM rust:1.85-bookworm AS builder

WORKDIR /src
COPY . .
RUN cargo build --release --package shelfbox

# Lightweight sandbox for manually verifying shelfbox workflows. This target
# deliberately excludes VHS, Chromium, ffmpeg, and the recorder fonts.
FROM debian:bookworm-slim AS verify

RUN apt-get update \
    && apt-get install --yes --no-install-recommends bash git \
    && mkdir -p /home/demo \
    && chmod 0777 /home/demo \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/shelfbox /usr/local/bin/shelfbox

WORKDIR /demo

# GIF recorder. Keep this separate from `verify` so its VHS/Chromium/ffmpeg
# dependencies are only pulled when a README recording is requested.
FROM ghcr.io/charmbracelet/vhs:v0.11.0 AS record

USER root
RUN apt-get update \
    && apt-get install --yes --no-install-recommends git \
    && mkdir -p /home/demo \
    && chmod 0777 /home/demo \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/shelfbox /usr/local/bin/shelfbox

WORKDIR /demo
