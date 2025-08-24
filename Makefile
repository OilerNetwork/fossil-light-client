.DEFAULT_GOAL := help

##@ Build

.PHONY: build
build: ## Build Rust code in release mode.
	cargo build --release

.PHONY: build-debug
build-debug: ## Build Rust code in debug mode.
	cargo build

.PHONY: build-cairo
build-cairo: ## Build Cairo code.
	scarb build

##@ Test

.PHONY: test
test: ## Test Rust code.
	cargo test --workspace --all-features

.PHONY: test-cairo
test-cairo: ## Test Cairo code with snforge.
	snforge test

##@ Linting

.PHONY: fmt
fmt: ## Format code with rustfmt.
	cargo +nightly fmt

.PHONY: clippy
clippy: ## Run clippy linter with project-specific settings.
	./scripts/clippy.sh

.PHONY: lint-codespell
lint-codespell: ensure-codespell ## Check for spelling mistakes.
	codespell

.PHONY: ensure-codespell
ensure-codespell:
	@if ! which codespell >/dev/null 2>&1; then \
		echo "codespell not found. Installing codespell..."; \
		if [ "$$(uname)" = "Darwin" ]; then \
			pip install codespell; \
		else \
			if command -v apt &> /dev/null; then \
				sudo apt-get update && (sudo apt-get install -y codespell || python3 -m pip install --user codespell); \
			elif command -v dnf &> /dev/null; then \
				sudo dnf install -y codespell || python3 -m pip install --user codespell; \
			elif command -v pacman &> /dev/null; then \
				sudo pacman -S --noconfirm codespell || python3 -m pip install --user codespell; \
			else \
				python3 -m pip install --user codespell; \
			fi; \
		fi; \
	else \
		echo "✅ codespell already installed"; \
	fi

.PHONY: lint
lint: fmt clippy lint-codespell fmt-cairo ## Run all linters.

.PHONY: fmt-cairo
fmt-cairo: ## Format Cairo code with scarb fmt.
	scarb fmt

##@ Pull Request

.PHONY: pr
pr: ## Prepare code for a pull request.
	make lint && \
	make test && \
	make fmt-cairo && \
	make test-cairo

##@ Help

.PHONY: help
help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

.PHONY: setup setup-rust setup-foundry setup-risc0 setup-starknet setup-platform init-repo

setup: setup-rust setup-foundry setup-risc0 setup-starknet setup-platform init-repo
	@echo "✅ All dependencies installed successfully!"

setup-rust:
	@echo "🔧 Checking Rust installation..."
	@if ! command -v rustup &> /dev/null; then \
		echo "Installing Rust..."; \
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
	else \
		echo "✅ Rust already installed"; \
	fi
	@if ! rustup toolchain list | grep -q "nightly"; then \
		echo "Installing Rust nightly..."; \
		rustup toolchain install nightly; \
		rustup default nightly; \
	else \
		echo "✅ Rust nightly already installed"; \
	fi

setup-foundry:
	@echo "🔧 Checking Foundry installation..."
	@if ! command -v forge &> /dev/null; then \
		echo "Installing Foundry..."; \
		curl -L https://foundry.paradigm.xyz | bash; \
		${HOME}/.foundry/bin/foundryup; \
	else \
		echo "✅ Foundry already installed"; \
	fi

setup-risc0:
	@echo "🔧 Checking Risc0 installation..."
	@if ! command -v rzup &> /dev/null; then \
		echo "Installing Risc0..."; \
		curl -L https://risczero.com/install | bash && rzup; \
	else \
		echo "✅ Risc0 already installed"; \
	fi

setup-starknet:
	@echo "🔧 Checking Starknet toolchain..."

	@if ! command -v asdf &> /dev/null; then \
		echo "Installing asdf (required for Starknet tools)..."; \
		git clone https://github.com/asdf-vm/asdf.git ~/.asdf --branch v0.12.0; \
		echo '. "$$HOME/.asdf/asdf.sh"' >> ~/.bashrc; \
		echo '. "$$HOME/.asdf/completions/asdf.bash"' >> ~/.bashrc; \
		. "$$HOME/.asdf/asdf.sh"; \
	else \
		echo "✅ asdf already installed"; \
	fi

	@. "$$HOME/.asdf/asdf.sh" && \
	if ! asdf which scarb &> /dev/null; then \
		echo "Installing scarb 2.9.4..."; \
		asdf install scarb 2.9.4; \
	else \
		echo "✅ scarb already installed"; \
	fi

	@. "$$HOME/.asdf/asdf.sh" && \
	if ! asdf which snforge &> /dev/null; then \
		echo "Installing starknet-foundry 0.37..."; \
		asdf install starknet-foundry 0.37; \
	else \
		echo "✅ starknet-foundry already installed"; \
	fi

	@. "$$HOME/.asdf/asdf.sh" && \
	if ! asdf plugin list | grep -q "starkli"; then \
		echo "Adding starkli plugin..."; \
		asdf plugin add starkli; \
	else \
		echo "✅ starkli plugin already added"; \
	fi

	@. "$$HOME/.asdf/asdf.sh" && \
	if ! asdf which starkli &> /dev/null; then \
		echo "Installing starkli..."; \
		asdf install starkli latest; \
	else \
		echo "✅ starkli already installed"; \
	fi

setup-platform:
	@echo "🔧 Checking platform-specific requirements..."
	@if [ "$$(uname)" = "Darwin" ]; then \
		echo "Checking macOS dependencies..."; \
		if ! command -v python3 &> /dev/null; then \
			echo "Installing Python..."; \
			brew install python; \
		else \
			echo "✅ Python already installed"; \
		fi; \
		if ! command -v gettext &> /dev/null; then \
			echo "Installing gettext..."; \
			brew install gettext; \
		else \
			echo "✅ gettext already installed"; \
		fi; \
		if ! grep -q "/usr/local/opt/python/libexec/bin" ~/.zshrc 2>/dev/null && ! grep -q "/usr/local/opt/python/libexec/bin" ~/.bash_profile 2>/dev/null; then \
			echo "Adding Python path to shell config..."; \
			echo 'export PATH="/usr/local/opt/python/libexec/bin:$$PATH"' >> ~/.zshrc; \
			echo 'export PATH="/usr/local/opt/python/libexec/bin:$$PATH"' >> ~/.bash_profile; \
		else \
			echo "✅ Python path already in shell config"; \
		fi; \
	else \
		echo "✅ No additional packages needed for Linux"; \
	fi

init-repo:
	@echo "🔧 Checking repository initialization..."
	@if [ -z "$$(git submodule status | grep -v '^ ')" ]; then \
		echo "Initializing git submodules..."; \
		git submodule update --init --recursive; \
	else \
		echo "✅ Git submodules already initialized"; \
	fi
