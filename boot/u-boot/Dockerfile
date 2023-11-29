# This should be version `jammy-20230804-25Aug2023`.
# We use the SHA256 hash here for improved traceability.
FROM trini/u-boot-gitlab-ci-runner@sha256:42925546cba6b1b0e819e91fb0269f0b700d72e4a2a5faf5d3102ec130fb859d

# Enable reproducible builds by setting a fixed timestamp.
# This is Wed Nov 22 2023 00:00:00 GMT+0000.
ENV SOURCE_DATE_EPOCH 1700611200

# Download U-Boot sources, verify their checksum, and unpack them.
# We are using U-Boot version 2023.10 here.
WORKDIR /home/uboot
COPY ./steps/00-prepare.sh .
RUN ./00-prepare.sh

# Build U-Boot binaries.
WORKDIR /home/uboot/u-boot-2023.10
COPY ./configs/* configs/
COPY ./steps/01-build.sh .
RUN ./01-build.sh

# Build boot scripts.
COPY ./steps/02-scripts.sh .
COPY ./scripts scripts
RUN ./02-scripts.sh
