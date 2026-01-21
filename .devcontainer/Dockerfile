FROM rust:1.92-bookworm

# Install basic development tools, PostgreSQL client
RUN apt-get update && apt-get install -y --no-install-recommends \
    less \
    git \
    procps \
    sudo \
    zsh \
    unzip \
    jq \
    postgresql-client \
    postgresql \
    postgresql-contrib \
    htop \
    libssl-dev \
    pkg-config \
    curl \
    ca-certificates \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

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

# Switch to non-root user for tool installation
USER ${USERNAME}

# Configure Cargo and NPM paths
ENV CARGO_HOME=/home/${USERNAME}/.cargo
ENV PATH=$CARGO_HOME/bin:$PATH


# Install Rust toolchain components
RUN rustup component add rustfmt clippy rust-analyzer

# Install Rust development tools
RUN cargo install sqlx-cli --no-default-features --features postgres,rustls && \
    cargo install cargo-watch just
# Set the default shell to zsh
ENV SHELL=/bin/zsh
ENV EDITOR=nano
ENV VISUAL=nano

# Install zsh with powerline10k theme
ARG ZSH_IN_DOCKER_VERSION=1.2.1
RUN sh -c "$(wget -O- https://github.com/deluan/zsh-in-docker/releases/download/v${ZSH_IN_DOCKER_VERSION}/zsh-in-docker.sh)" -- \
    -p git \
    -a "export PROMPT_COMMAND='history -a' && export HISTFILE=/commandhistory/.bash_history" \
    -x

# Install Claude Code CLI and gemini cli
RUN curl -fsSL https://claude.ai/install.sh | bash && \
    curl -LsSf https://astral.sh/uv/install.sh | sh

# Install Node.js (required for npm packages like Claude Code CLI and Gemini CLI)
# RUN curl -fsSL https://deb.nodesource.com/setup_25.x | bash - && \
#     apt-get install -y nodejs && \
#     apt-get clean && rm -rf /var/lib/apt/lists/* \
#     npm install -g @google/gemini-cli
