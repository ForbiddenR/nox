#!/usr/bin/env bash
# Example agent script that works with the real runner
# Usage: ./example-agent.sh
# Reads prompt from stdin, outputs JSON events to stdout

set -euo pipefail

# Read the prompt from stdin
read -r PROMPT

# Output JSON events (one per line)
echo '{"type":"text","data":"Received prompt: '"$PROMPT"'"}'
sleep 0.5

echo '{"type":"text","data":"\n\nAnalyzing the request..."}'
sleep 0.8

echo '{"type":"tool_start","data":{"tool":"list_files","args":{"path":"."}}}'
sleep 0.5

echo '{"type":"tool_end","data":{"tool":"list_files","result":"Found 15 files"}}'
sleep 0.3

echo '{"type":"text","data":"\n\nBased on my analysis, I recommend the following approach..."}'
sleep 0.5

echo '{"type":"text","data":"\n\n1. Review the existing code structure\n2. Identify the key components\n3. Make targeted improvements"}'
sleep 0.5

echo '{"type":"text","data":"\n\nCompleted!"}'
