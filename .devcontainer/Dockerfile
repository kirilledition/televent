FROM rust:1.75-bookworm

ARG TZ
ENV TZ="$TZ"

ARG CLAUDE_CODE_VERSION=latest

# Install basic development tools, PostgreSQL client, Node.js for Tailwind
RUN apt-get update && apt-get install -y --no-install-recommends \
    less \
    git \
    procps \
    sudo \
    fzf \
    zsh \
    man-db \
    unzip \
    gnupg2 \
    gh \
    iptables \
    ipset \
    iproute2 \
    dnsutils \
    aggregate \
    jq \
    nano \
    vim \
    postgresql-client \
    libssl-dev \
    pkg-config \
    curl \
    ca-certificates \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# Install Node.js 20 for Tailwind CSS (minimal installation)
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

# Create non-root user for development
ARG USERNAME=rustdev
RUN useradd -m -s /bin/zsh -G sudo ${USERNAME} && \
    echo "${USERNAME} ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Ensure user has access to cargo and npm directories
RUN mkdir -p /usr/local/share/npm-global && \
    chown -R ${USERNAME}:${USERNAME} /usr/local/share

# Persist bash history
RUN mkdir -p /commandhistory && \
    touch /commandhistory/.bash_history && \
    chown -R ${USERNAME}:${USERNAME} /commandhistory

# Set `DEVCONTAINER` environment variable to help with orientation
ENV DEVCONTAINER=true

# Create workspace and config directories and set permissions
RUN mkdir -p /workspace /home/${USERNAME}/.claude /home/${USERNAME}/.cargo && \
    chown -R ${USERNAME}:${USERNAME} /workspace /home/${USERNAME}/.claude /home/${USERNAME}/.cargo

WORKDIR /workspace

ARG GIT_DELTA_VERSION=0.18.2
RUN ARCH=$(dpkg --print-architecture) && \
    wget "https://github.com/dandavison/delta/releases/download/${GIT_DELTA_VERSION}/git-delta_${GIT_DELTA_VERSION}_${ARCH}.deb" && \
    dpkg -i "git-delta_${GIT_DELTA_VERSION}_${ARCH}.deb" && \
    rm "git-delta_${GIT_DELTA_VERSION}_${ARCH}.deb"

# Switch to non-root user for tool installation
USER ${USERNAME}

# Configure Cargo and NPM paths
ENV CARGO_HOME=/home/${USERNAME}/.cargo
ENV PATH=$CARGO_HOME/bin:$PATH
ENV NPM_CONFIG_PREFIX=/usr/local/share/npm-global
ENV PATH=$PATH:/usr/local/share/npm-global/bin

# Install Rust toolchain components
RUN rustup component add rustfmt clippy rust-analyzer

# Install Rust development tools
RUN cargo install sqlx-cli --no-default-features --features postgres,rustls && \
    cargo install cargo-watch && \
    cargo install dioxus-cli && \
    cargo install just

# Set the default shell to zsh
ENV SHELL=/bin/zsh
ENV EDITOR=nano
ENV VISUAL=nano

# Install zsh with powerline10k theme
ARG ZSH_IN_DOCKER_VERSION=1.2.0
RUN sh -c "$(wget -O- https://github.com/deluan/zsh-in-docker/releases/download/v${ZSH_IN_DOCKER_VERSION}/zsh-in-docker.sh)" -- \
    -p git \
    -p fzf \
    -p rust \
    -a "source /usr/share/doc/fzf/examples/key-bindings.zsh" \
    -a "source /usr/share/doc/fzf/examples/completion.zsh" \
    -a "export PROMPT_COMMAND='history -a' && export HISTFILE=/commandhistory/.bash_history" \
    -x

# Install Claude Code CLI and Tailwind CSS
RUN npm install -g @anthropic-ai/claude-code@${CLAUDE_CODE_VERSION} tailwindcss

# Copy and set up firewall script
USER root
COPY init-firewall.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/init-firewall.sh && \
    echo "${USERNAME} ALL=(root) NOPASSWD: /usr/local/bin/init-firewall.sh" > /etc/sudoers.d/${USERNAME}-firewall && \
    chmod 0440 /etc/sudoers.d/${USERNAME}-firewall

USER ${USERNAME}