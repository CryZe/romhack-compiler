FROM ubuntu:14.04

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    cmake3 \
    g++-multilib \
    libc6-dev \
    make \
    pkg-config \
    git-all \
    libgtk-3-dev

COPY xargo.sh /
RUN bash /xargo.sh
