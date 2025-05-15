#!/bin/bash

# Test script to verify command injection prevention
echo "Testing SSHS command injection prevention..."

# Remove any existing output file
rm -f /tmp/output

# Test with the dangerous SSH config
# This should fail due to validation
echo "Testing with dangerous SSH config..."
source $HOME/.cargo/env
cd /Users/imrellx/code/sshs

# Run with the dangerous config - this should fail
./target/debug/sshs -c ./test_configs/dangerous_ssh_config --template "echo 'Connecting to: {{name}}'"

# Check if the dangerous command was executed
if [ -f /tmp/output ]; then
    echo "VULNERABILITY: Dangerous command was executed!"
    cat /tmp/output
    rm -f /tmp/output
    exit 1
else
    echo "SUCCESS: Dangerous command was prevented"
fi

echo "All tests passed!"
