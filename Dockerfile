# -----------------
# Cargo Build Stage
# -----------------

FROM rust:latest as cargo-build

# Building dependencies
WORKDIR /usr/src/app
COPY Cargo.lock Cargo.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

# Building the actual app
COPY Siostam.example.toml build.rs .env.example ./
COPY ./src src
RUN cargo build --release
RUN cargo install --path . --verbose

# -----------------
# Final Stage
# -----------------

FROM siostam/ngx-siostam:0.2

COPY --from=cargo-build /usr/local/cargo/bin/siostam /opt

RUN apt-get update && apt-get install -y libssl1.1 graphviz
RUN fdp -V

WORKDIR /opt
# RUN mkdir touch /opt/data/output.dot.svg
CMD ["/opt/siostam", "server"]
EXPOSE 4300/tcp
