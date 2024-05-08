#########################################################################################
# Rust Build Environment
#
# The image contains everything necessary to (cross)-compile our Rust crates.
#########################################################################################
FROM debian:latest as build-env

COPY bakery/layers/build-env/00-base.sh /tmp/rugpi-docker/00-base.sh
RUN /tmp/rugpi-docker/00-base.sh

# Install Rust toolchain.
ENV RUSTUP_HOME="/usr/local/rustup" \
    CARGO_HOME="/usr/local/cargo" \
    PATH="/usr/local/cargo/bin:${PATH}" \
    RUST_VERSION="1.76.0"

COPY bakery/layers/build-env/10-rust.sh /tmp/rugpi-docker/10-rust.sh
RUN /tmp/rugpi-docker/10-rust.sh

# Install libraries and configure for cross compilation.
ENV PKG_CONFIG_SYSROOT_DIR="/"

COPY bakery/layers/build-env/20-libs.sh /tmp/rugpi-docker/20-libs.sh
RUN /tmp/rugpi-docker/20-libs.sh

ENV RUGPI_BUILD_ENV="true"

WORKDIR /project
CMD /usr/bin/zsh


#########################################################################################
# `cargo-chef` Planner
#
# We are using `cargo-chef` to speed up builds of the image.
#########################################################################################
FROM build-env AS planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json


#########################################################################################
# `cargo-chef` Builder
#
# Build all Rust crates with `cargo-chef`.
#########################################################################################
FROM build-env AS builder 

COPY --from=planner /project/recipe.json recipe.json
COPY bakery/layers/builder/00-prepare.sh /tmp/rugpi-docker/00-prepare.sh
RUN /tmp/rugpi-docker/00-prepare.sh

COPY . .
COPY bakery/layers/builder/10-build.sh /tmp/rugpi-docker/10-build.sh
RUN /tmp/rugpi-docker/10-build.sh


#########################################################################################
# Rugpi Bakery Image
#
# Now combine everything in the `rugpi-bakery` image.
#########################################################################################
FROM debian:latest AS bakery

ARG TARGETPLATFORM

COPY bakery/layers/bakery/00-base.sh /tmp/rugpi-docker/00-base.sh
RUN /tmp/rugpi-docker/00-base.sh

RUN mkdir -p /usr/share/rugpi \
    && cd /usr/share/rugpi \
    && wget -O rpi-eeprom-master.zip https://github.com/raspberrypi/rpi-eeprom/archive/refs/heads/master.zip \
    && unzip rpi-eeprom-master.zip \
    && mv rpi-eeprom-master rpi-eeprom \
    && rm -f rpi-eeprom-master.zip

COPY bakery/repositories /usr/share/rugpi/repositories
COPY boot /usr/share/rugpi/boot

COPY bakery/layers/bakery/10-setup.sh /tmp/rugpi-docker/10-setup.sh
RUN /tmp/rugpi-docker/10-setup.sh

COPY build/binaries /usr/share/rugpi/binaries

WORKDIR /project

RUN if [ "$TARGETPLATFORM" = "linux/amd64" ]; then \
        cp /usr/share/rugpi/binaries/x86_64-unknown-linux-musl/rugpi-bakery /usr/local/bin; \
    elif [ "$TARGETPLATFORM" = "linux/arm64" ]; then \
        cp /usr/share/rugpi/binaries/aarch64-unknown-linux-musl/rugpi-bakery /usr/local/bin; \
    else  \
        echo "Unsupported platform ${TARGETPLATFORM}."; \
        exit 1; \
    fi

ENTRYPOINT ["/usr/local/bin/rugpi-bakery"]