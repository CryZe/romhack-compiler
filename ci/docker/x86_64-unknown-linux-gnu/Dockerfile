FROM ubuntu:14.04

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    cmake \
    g++-multilib \
    libc6-dev \
    make \
    pkg-config \
    git-all

COPY ../xargo.sh /
RUN bash /xargo.sh
