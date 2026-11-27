# Portal PPDB Online: image kecil, satu binary Rust + aset statis.
# ---- 1. CSS Tailwind dari template View ----
FROM debian:bookworm-slim AS css
ARG TAILWIND_VERSION=v3.4.17
RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /src
RUN curl -fsSLo /usr/local/bin/tailwindcss https://github.com/tailwindlabs/tailwindcss/releases/download/${TAILWIND_VERSION}/tailwindcss-linux-x64 \
 && chmod +x /usr/local/bin/tailwindcss
COPY tailwind.config.js ./
COPY styles ./styles
COPY templates ./templates
RUN tailwindcss -i styles/app.css -o app.css --minify

# ---- 2. Binary Rust (Model, View, Controller) ----
FROM rust:1-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates
RUN cargo build --release --locked

# ---- 3. Runtime ----
FROM debian:bookworm-slim
RUN useradd --uid 10001 --no-create-home --shell /usr/sbin/nologin app
WORKDIR /app
COPY --from=build /src/target/release/ppdb-portal /app/ppdb-portal
COPY static ./static
COPY --from=css /src/app.css ./static/app.css
USER 10001
ENV PORT=8080 STATIC_DIR=/app/static
EXPOSE 8080
CMD ["/app/ppdb-portal"]
