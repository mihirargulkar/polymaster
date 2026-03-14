#!/bin/bash
set -e

echo "Building Polymaster Docker Image..."
docker build -t polymaster-bot .

echo "Build successful! To test locally, run:"
echo "docker run --env-file .env polymaster-bot"
echo "---"
echo "To deploy to Render:"
echo "1. Connect your Github Repo to Render."
echo "2. Create a new 'Background Worker'"
echo "3. Render will auto-detect the Dockerfile and deploy it continuously."
