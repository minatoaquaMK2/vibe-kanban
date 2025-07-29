#!/bin/bash

set -e  # Exit on any error

# Function to detect platform and architecture
detect_platform() {
    local os=$(uname -s)
    local arch=$(uname -m)

    case "$os" in
        "Darwin")
            # macOS - detect if running on Apple Silicon or Intel
            if [[ "$arch" == "arm64" ]]; then
                echo "macos-arm64"
            elif [[ "$arch" == "x86_64" ]]; then
                # Check if running under Rosetta
                if [[ $(sysctl -in sysctl.proc_translated 2>/dev/null || echo "0") == "1" ]]; then
                    echo "macos-arm64"
                else
                    echo "macos-x64"
                fi
            else
                echo "macos-x64"  # fallback
            fi
            ;;
        "Linux")
            case "$arch" in
                "x86_64")
                    echo "linux-x64"
                    ;;
                "aarch64"|"arm64")
                    echo "linux-arm64"
                    ;;
                *)
                    echo "linux-x64"  # fallback
                    ;;
            esac
            ;;
        "MINGW"*|"MSYS"*|"CYGWIN"*)
            # Windows
            case "$arch" in
                "x86_64")
                    echo "windows-x64"
                    ;;
                "aarch64"|"arm64")
                    echo "windows-arm64"
                    ;;
                *)
                    echo "windows-x64"  # fallback
                    ;;
            esac
            ;;
        *)
            echo "Unsupported OS: $os" >&2
            exit 1
            ;;
    esac
}

# Get platform directory
PLATFORM_DIR=$(detect_platform)
echo "🔍 Detected platform: $PLATFORM_DIR"

echo "🧹 Cleaning previous builds..."
rm -rf npx-cli/dist
mkdir -p "npx-cli/dist/$PLATFORM_DIR"

echo "🔨 Building frontend..."
(cd frontend && npm run build)

echo "🔨 Building Rust binaries..."
cargo build --release --manifest-path backend/Cargo.toml
cargo build --release --bin mcp_task_server --manifest-path backend/Cargo.toml

echo "📦 Creating distribution package..."

# Determine binary extension based on platform
if [[ "$PLATFORM_DIR" == windows-* ]]; then
    BINARY_EXT=".exe"
else
    BINARY_EXT=""
fi

# Copy the main binary
cp "target/release/vibe-kanban$BINARY_EXT" "vibe-kanban$BINARY_EXT"
cp "target/release/mcp_task_server$BINARY_EXT" "vibe-kanban-mcp$BINARY_EXT"

zip "vibe-kanban.zip" "vibe-kanban$BINARY_EXT"
zip "vibe-kanban-mcp.zip" "vibe-kanban-mcp$BINARY_EXT"

rm "vibe-kanban$BINARY_EXT" "vibe-kanban-mcp$BINARY_EXT"

mv vibe-kanban.zip "npx-cli/dist/$PLATFORM_DIR/vibe-kanban.zip"
mv vibe-kanban-mcp.zip "npx-cli/dist/$PLATFORM_DIR/vibe-kanban-mcp.zip"

echo "✅ NPM package ready!"
echo "📁 Files created:"
echo "   - npx-cli/dist/$PLATFORM_DIR/vibe-kanban.zip"
echo "   - npx-cli/dist/$PLATFORM_DIR/vibe-kanban-mcp.zip"