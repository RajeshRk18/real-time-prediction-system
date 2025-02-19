FROM rust:1.75-slim as builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    libopenblas-base \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/inference_server /usr/local/bin/
COPY model/MarketRegimeModel.onnx /model/

CMD ["inference_server"]