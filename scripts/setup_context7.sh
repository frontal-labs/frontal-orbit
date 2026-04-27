#!/bin/bash

# Context7 Setup Script for Orbit
# This script helps set up Context7 MCP server for Orbit

set -e

echo "🚀 Setting up Context7 for Orbit..."

# Check if Node.js is installed
if ! command -v node &> /dev/null; then
    echo "❌ Node.js is required but not installed. Please install Node.js 18 or higher."
    echo "Visit: https://nodejs.org/"
    exit 1
fi

# Check Node.js version
NODE_VERSION=$(node -v | cut -d'v' -f2 | cut -d'.' -f1)
if [ "$NODE_VERSION" -lt 18 ]; then
    echo "❌ Node.js version 18 or higher is required. Current version: $(node -v)"
    exit 1
fi

echo "✅ Node.js version check passed: $(node -v)"

# Check if npm/npx is available
if ! command -v npx &> /dev/null; then
    echo "❌ npx is required but not found."
    exit 1
fi

echo "✅ npx is available"

# Check if Context7 API key is set
if [ -z "$CONTEXT7_API_KEY" ]; then
    echo ""
    echo "📝 Context7 API Key Setup"
    echo "=========================="
    echo "1. Visit https://context7.com/dashboard"
    echo "2. Sign up or log in"
    echo "3. Generate an API key"
    echo "4. Set the environment variable:"
    echo ""
    echo "   export CONTEXT7_API_KEY=your_api_key_here"
    echo ""
    echo "   Or add it to your shell profile (~/.bashrc, ~/.zshrc, etc.)"
    echo ""
    read -p "Do you have a Context7 API key? (y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Please get an API key from https://context7.com/dashboard and run this script again."
        exit 1
    fi
    
    read -p "Enter your Context7 API key: " -s API_KEY
    echo
    export CONTEXT7_API_KEY="$API_KEY"
    
    # Add to shell profile
    echo ""
    read -p "Add API key to ~/.zshrc? (y/n): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "export CONTEXT7_API_KEY=\"$API_KEY\"" >> ~/.zshrc
        echo "✅ API key added to ~/.zshrc"
    fi
else
    echo "✅ Context7 API key is set"
fi

# Test Context7 MCP server
echo ""
echo "🧪 Testing Context7 MCP server..."
if npx -y @upstash/context7-mcp@latest --help &> /dev/null; then
    echo "✅ Context7 MCP server is accessible"
else
    echo "❌ Failed to access Context7 MCP server"
    exit 1
fi

# Check Orbit configuration
echo ""
echo "🔧 Checking Orbit configuration..."
ORBIT_CONFIG="$HOME/.orbit/settings.json"
if [ -f "$ORBIT_CONFIG" ]; then
    echo "✅ Orbit configuration found at $ORBIT_CONFIG"
else
    echo "ℹ️  No Orbit configuration found. Using project configuration."
fi

# Check if context7.json exists
if [ -f "context7.json" ]; then
    echo "✅ context7.json configuration found"
else
    echo "❌ context7.json not found in current directory"
    exit 1
fi

# Verify .orbit.json has context7 configuration
if grep -q "context7" .orbit.json; then
    echo "✅ Context7 is configured in .orbit.json"
else
    echo "❌ Context7 not found in .orbit.json"
    exit 1
fi

echo ""
echo "🎉 Context7 setup completed successfully!"
echo ""
echo "Next steps:"
echo "1. Restart your Orbit session"
echo "2. Use 'use context7' in your prompts to fetch up-to-date documentation"
echo "3. Example: 'Create a React component with hooks. use context7'"
echo ""
echo "For more information, see:"
echo "- Context7 documentation: https://github.com/upstash/context7"
echo "- Orbit MCP guide: docs/MCP.md"
echo "- Context7 configuration: context7.json"
