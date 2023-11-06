FROM registry.fedoraproject.org/fedora:38

RUN dnf install -y make automake gcc gcc-c++
RUN dnf install -y systemd-devel

RUN curl https://sh.rustup.rs -sSf | sh -s -- --profile minimal -y
ENV PATH $PATH:/root/.cargo/bin
