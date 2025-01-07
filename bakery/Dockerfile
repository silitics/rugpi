FROM debian:latest

ARG TARGETPLATFORM

RUN apt-get -y update \
    && apt-get -y install \
        btrfs-progs \
        curl \
        dosfstools \
        fdisk \
        file \
        git \
        gpg \
        mmdebstrap \
        mtools \
        proot \
        python3 \
        python3-venv \
        qemu-system-arm \
        qemu-system-x86 \
        qemu-utils \
        wget \
        xz-utils \
        zip \
        zsh \
    && apt-get -y clean \
    && rm -rf /var/lib/apt/lists/* \
    && wget -O /etc/zsh/zshrc https://git.grml.org/f/grml-etc-core/etc/zsh/zshrc \
    && touch /root/.zshrc

# Tools required to run build Yocto images with Rugpi.
RUN apt-get -y update \
    && apt-get -y install \
        build-essential \
        chrpath \
        cpio \
        debianutils \
        diffstat \
        file \
        gawk \
        gcc \
        git \
        iputils-ping \
        libacl1 \
        liblz4-tool \
        locales \
        python3 \
        python3-git \
        python3-jinja2 \
        python3-pexpect \
        python3-pip \
        python3-subunit \
        socat \
        texinfo \
        unzip \
        wget \
        xz-utils \
        zstd \
    && apt-get -y clean \
    && rm -rf /var/lib/apt/lists/*

# Add a `build` user for unprivileged builds.
RUN useradd -ms /usr/bin/zsh build
USER build
RUN touch ~/.zshrc
USER root

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

# Properly set up locale, required by some tools.
RUN locale-gen en_US.UTF-8 \
    && sed -i '/en_US.UTF-8/s/^# //g' /etc/locale.gen \
    && locale-gen

ENV LANGUAGE=en_US.UTF-8 LANG=en_US.UTF-8 LC_ALL=en_US.UTF-8

COPY boot /usr/share/rugpi/boot
COPY bakery/repositories /usr/share/rugpi/repositories
COPY bakery/templates /usr/share/rugpi/templates

COPY build/binaries /usr/share/rugpi/binaries

WORKDIR /project

RUN if [ "$TARGETPLATFORM" = "linux/amd64" ]; then \
        cp /usr/share/rugpi/binaries/x86_64-unknown-linux-musl/rugpi-bakery /usr/local/bin; \
    elif [ "$TARGETPLATFORM" = "linux/arm64" ] || [ "$TARGETPLATFORM" = "linux/arm64/v8" ]; then \
        cp /usr/share/rugpi/binaries/aarch64-unknown-linux-musl/rugpi-bakery /usr/local/bin; \
    else  \
        echo "Unsupported platform ${TARGETPLATFORM}."; \
        exit 1; \
    fi

ENTRYPOINT ["/usr/local/bin/rugpi-bakery"]