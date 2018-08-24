FROM japaric/x86_64-unknown-linux-gnu

RUN apt-get update && \
    apt-get install -y software-properties-common && \
    add-apt-repository -y ppa:george-edison55/precise-backports && \
    apt-get update && \
    apt-get install -y git-all cmake
