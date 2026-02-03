#!/bin/bash
#
# Run unit tests for VoidMei utility classes.
#
# Usage:
#   ./script/test.sh              # Run all tests
#   ./script/test.sh atmosphere   # Run AtmosphereModel tests only
#   ./script/test.sh piston       # Run PistonPowerModel tests only
#

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== VoidMei Unit Tests ===${NC}"
echo ""

# Ensure bin directory exists
mkdir -p bin

# Compile main sources if needed
if [ ! -f "bin/prog/util/AtmosphereModel.class" ] || \
   [ "src/prog/util/AtmosphereModel.java" -nt "bin/prog/util/AtmosphereModel.class" ]; then
    echo "Compiling main sources..."
    find src -name "*.java" > sources.txt
    javac -encoding UTF-8 -d bin -classpath 'dep/*' @sources.txt
    rm sources.txt
    echo ""
fi

# Compile test sources
echo "Compiling tests..."
javac -encoding UTF-8 -d bin -classpath bin test/*.java
echo ""

# Track overall results
TOTAL_PASSED=0
TOTAL_FAILED=0

run_test() {
    local test_name=$1
    local class_name=$2

    echo -e "${YELLOW}Running $test_name...${NC}"
    echo ""

    if java -classpath bin "$class_name"; then
        echo ""
        echo -e "${GREEN}$test_name: PASSED${NC}"
        return 0
    else
        echo ""
        echo -e "${RED}$test_name: FAILED${NC}"
        return 1
    fi
}

# Determine which tests to run
case "${1:-all}" in
    atmosphere|atm)
        if run_test "AtmosphereModel Tests" "TestAtmosphereModel"; then
            ((TOTAL_PASSED++)) || true
        else
            ((TOTAL_FAILED++)) || true
        fi
        ;;
    piston|power)
        if run_test "PistonPowerModel Tests" "TestPistonPowerModel"; then
            ((TOTAL_PASSED++)) || true
        else
            ((TOTAL_FAILED++)) || true
        fi
        ;;
    all|*)
        echo "=========================================="
        if run_test "AtmosphereModel Tests" "TestAtmosphereModel"; then
            ((TOTAL_PASSED++)) || true
        else
            ((TOTAL_FAILED++)) || true
        fi

        echo ""
        echo "=========================================="
        if run_test "PistonPowerModel Tests" "TestPistonPowerModel"; then
            ((TOTAL_PASSED++)) || true
        else
            ((TOTAL_FAILED++)) || true
        fi
        ;;
esac

# Summary
echo ""
echo "=========================================="
echo -e "${YELLOW}=== Test Summary ===${NC}"
echo -e "Test suites passed: ${GREEN}$TOTAL_PASSED${NC}"
echo -e "Test suites failed: ${RED}$TOTAL_FAILED${NC}"

if [ $TOTAL_FAILED -gt 0 ]; then
    echo ""
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
else
    echo ""
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
fi
