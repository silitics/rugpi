FROM debian:latest

ARG TARGETPLATFORM

COPY bakery/layers/bakery/00-base.sh /tmp/rugpi-docker/00-base.sh
RUN /tmp/rugpi-docker/00-base.sh

RUN mkdir -p /usr/share/rugpi \
    && cd /usr/share/rugpi \
    && wget -O rpi-eeprom-master.zip https://github.com/raspberrypi/rpi-eeprom/archive/refs/heads/master.zip \
    && unzip rpi-eeprom-master.zip \
    && mv rpi-eeprom-master rpi-eeprom \
    && rm -f rpi-eeprom-master.zip

RUN mkdir -p /usr/share/rugpi/pi \
    && cd /usr/share/rugpi/pi \
    && wget -O rpi-firmware.zip https://github.com/raspberrypi/firmware/archive/refs/heads/stable.zip \
    && unzip rpi-firmware.zip \
    && mv firmware-stable/boot firmware \
    && rm -rf firmware-stable \
    && rm -f firmware/kernel*.img

COPY boot /usr/share/rugpi/boot

COPY bakery/layers/bakery/10-setup.sh /tmp/rugpi-docker/10-setup.sh
RUN /tmp/rugpi-docker/10-setup.sh

COPY bakery/repositories /usr/share/rugpi/repositories
COPY bakery/templates /usr/share/rugpi/templates

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