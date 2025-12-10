#!/bin/bash
# Build Rust documentation for deployment

set -e

echo "🔨 Building Rust documentation..."

# Navigate to backend directory
cd "$(dirname "$0")/.." || exit 1

# Clean previous builds
echo "🧹 Cleaning previous documentation..."
cargo clean --doc

# Build documentation (no dependencies to keep it focused on our code)
echo "📚 Generating documentation..."
cargo doc --no-deps

# Check if documentation was generated
# The main index might be at target/doc/index.html or target/doc/vault_backend/index.html
if [ ! -f "target/doc/index.html" ] && [ ! -f "target/doc/vault_backend/index.html" ]; then
    echo "❌ Error: Documentation not generated!"
    exit 1
fi

# Find the main index file
if [ -f "target/doc/index.html" ]; then
    MAIN_INDEX="target/doc/index.html"
else
    MAIN_INDEX="target/doc/vault_backend/index.html"
fi

echo "✅ Documentation built successfully!"
echo "📍 Location: $(pwd)/$MAIN_INDEX"
echo ""
echo "To view locally:"
echo "  cd target/doc && python3 -m http.server 8000"
echo ""
echo "To deploy:"
echo "  1. Copy target/doc/* to your hosting provider"
echo "  2. Or use the GitHub Actions workflow in .github/workflows/docs.yml"
echo "  3. Or use Netlify/Vercel with the provided config files"
