FROM registry.fedoraproject.org/fedora:40

RUN dnf install -y make automake gcc gcc-c++
RUN dnf install -y systemd-devel openssl-devel

RUN curl https://sh.rustup.rs -sSf | sh -s -- --profile minimal -y
ENV PATH $PATH:/root/.cargo/bin
