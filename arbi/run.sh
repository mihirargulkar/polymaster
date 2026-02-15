#!/bin/bash
set -e

# Change to script directory
cd "$(dirname "$0")"

# Load .env if present
if [ -f .env ]; then
  echo "Loading .env..."
  export $(grep -v '^#' .env | xargs)
else
  echo "⚠️  No .env file found. Using current environment."
fi

# Check for required keys
if [ -z "$GROQ_API_KEY" ]; then 
    echo "❌ GROQ_API_KEY is missing. Dependency discovery will fail."
fi

# Check live trading keys if --live is passed
if [[ "$*" == *"--live"* ]]; then
    MISSING=0
    if [ -z "$POLY_API_KEY" ]; then echo "❌ POLY_API_KEY missing"; MISSING=1; fi
    if [ -z "$POLY_API_SECRET" ]; then echo "❌ POLY_API_SECRET missing"; MISSING=1; fi
    if [ -z "$POLY_PASSPHRASE" ]; then echo "❌ POLY_PASSPHRASE missing"; MISSING=1; fi
    
    if [ $MISSING -eq 1 ]; then
        echo "🚨 Live trading requires all API keys."
        exit 1
    fi
fi

# Build (ensure latest code)
if [ ! -d "build" ]; then
    echo "Creating build directory..."
    mkdir build && cd build && cmake ..
else
    cd build
fi

echo "Compiling..."
make -j8
cd ..

# Run via caffeinate to prevent sleep (optional, good for bots)
echo "🚀 Starting ARBI..."
./build/arbi "$@"
