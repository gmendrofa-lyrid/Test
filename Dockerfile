FROM lukemathwalker/cargo-chef:latest AS chef
WORKDIR /app

FROM chef AS planner
COPY ./Cargo.toml ./Cargo.lock ./
COPY ./.sqlx ./.sqlx
COPY ./src ./src
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ENV SQLX_OFFLINE=1
ARG PDFIUM_VERSION=7749
ARG PDFIUM_PLATFORM=linux-x64
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && curl -L \
    "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/${PDFIUM_VERSION}/pdfium-${PDFIUM_PLATFORM}.tgz" \
    -o /tmp/pdfium.tgz \
    && tar -xzf /tmp/pdfium.tgz -C /tmp \
    && cp /tmp/lib/libpdfium.so /usr/local/lib/ \
    && ldconfig \
    && rm -rf /tmp/pdfium*

COPY --from=planner /app/recipe.json .
RUN cargo chef cook --release --recipe-path recipe.json

COPY migrations migrations
COPY . .
RUN cargo build --release \
    && mv ./target/release/snop_cockpit_be ./app

FROM debian:stable-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/lib/libpdfium.so /usr/local/lib/
RUN ldconfig

RUN mkdir -p file

COPY --from=builder /app/app /usr/local/bin/
COPY --from=builder /app/.env .
COPY --from=builder /app/migrations migrations

EXPOSE 9093

ENTRYPOINT ["/usr/local/bin/app"]
