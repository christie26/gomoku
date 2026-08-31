FROM registry.fedoraproject.org/fedora-toolbox:latest

# Install dependencies
RUN dnf install -y nix shadow-utils mesa-dri-drivers libX11 libXcursor libXrandr libXi && dnf clean all

# Pre-create /nix and make it fully writable by any unprivileged user
RUN mkdir -p /nix /etc/nix && \
    chmod -R 777 /nix && \
    echo "experimental-features = nix-command flakes" > /etc/nix/nix.conf
