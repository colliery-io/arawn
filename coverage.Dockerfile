FROM --platform=linux/amd64 rust:latest

RUN rustup toolchain install nightly \
    && rustup component add llvm-tools-preview --toolchain nightly \
    && cargo install cargo-llvm-cov

WORKDIR /src
