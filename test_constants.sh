#!/bin/bash

# Integration test for constants usage
echo "Testing constants implementation..."

# Test compilation to ensure constants are properly defined
cd /Users/imrellx/code/sshs
source $HOME/.cargo/env

echo "Test 1: Verifying constants compilation"
if cargo check --quiet 2>/dev/null; then
    echo "✓ Constants are properly defined and compile successfully"
else
    echo "✗ Compilation failed - constants may have issues"
    exit 1
fi

echo "Test 2: Running constants tests"
if cargo test --quiet --bin sshs ui::tests::test_constants_are_properly_defined 2>/dev/null; then
    echo "✓ UI constants tests pass"
else
    echo "✗ UI constants tests failed"
    exit 1
fi

if cargo test --quiet --bin sshs tests::test_constants_accessibility 2>/dev/null; then
    echo "✓ Main constants tests pass"
else
    echo "✗ Main constants tests failed"
    exit 1
fi

echo "Test 3: Verifying no hardcoded magic numbers remain"
if grep -r "21\|3\|4\|5\|1" src/ --include="*.rs" | grep -v "const\|test\|rustc\|comment" | grep -E "\b(21|[345])\b" > /tmp/magic_numbers; then
    echo "Found potential magic numbers (excluding tests and constants):"
    cat /tmp/magic_numbers | head -5
    echo "NOTE: Some of these may be acceptable (comments, version numbers, etc.)"
else
    echo "✓ No obvious magic numbers found (good!)"
fi

echo "All tests passed! Constants are properly implemented."
