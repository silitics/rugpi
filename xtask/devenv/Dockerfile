FROM rust:latest

COPY xtask/devenv/setup.sh .

RUN ./setup.sh && rm -f ./setup.sh

WORKDIR /project
