#!/bin/bash
# SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
# SPDX-License-Identifier: MIT

#!/bin/bash
# Serve the web version locally for testing
# This will start a local server at http://127.0.0.1:8080

echo "Starting local web server..."
echo "Open http://127.0.0.1:8080 in your browser"
echo "Press Ctrl+C to stop"
echo ""

trunk serve --open
