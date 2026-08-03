FROM registry.fedoraproject.org/fedora-toolbox:latest

# 1. Install base tools, Nix, and Ghostty terminfo
RUN dnf install -y \
    neovim \
    zsh \
    git \
    golang \
    python3-pip \
    ncurses \
    nix \
    sudo \
    && dnf clean all

# 2. Configure Nix globally for single-user mode with Flakes enabled
RUN mkdir -p /etc/nix && \
    echo "experimental-features = nix-command flakes" > /etc/nix/nix.conf && \
    echo "build-users-group =" >> /etc/nix/nix.conf && \
    echo "sandbox = false" >> /etc/nix/nix.conf && \
    echo "use-xdg-base-directories = true" >> /etc/nix/nix.conf

# 3. Copy flake files and pre-cache devShell dependencies into /nix/store
WORKDIR /tmp/build
COPY flake.nix flake.lock* ./
RUN nix print-dev-env . > /dev/null
WORKDIR /
RUN rm -rf /tmp/build

# 4. Make Nix store and DB directories fully accessible to non-root users
RUN chmod -R a+rwX /nix

# 5. Environment variables: force single-user mode and user-local profiles
ENV NIX_REMOTE=""
ENV TERM=xterm-256color

# Label explicitly as a toolbox-compatible container
LABEL com.github.containers.toolbox="true"
