FROM rust:latest

RUN apt-get update && apt-get install -y \
    libssl-dev pkg-config clang curl protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user for VS Code
RUN useradd -m vscode && usermod -aG sudo vscode

# Set work directory
WORKDIR /workspace

# Default shell
CMD ["/bin/bash"]
