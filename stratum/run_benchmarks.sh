#!/bin/bash

# Loka Stratum Critical Path Benchmarks Runner
# This script provides convenient access to run critical path benchmarks

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔧 Loka Stratum Critical Path Benchmarks${NC}"
echo "================================================"

# Function to run specific benchmark
run_benchmark() {
    local bench_name="$1"
    echo -e "${YELLOW}🚀 Running benchmark: ${bench_name}${NC}"
    cargo bench --bench critical_path "${bench_name}"
}

# Function to run all benchmarks
run_all_benchmarks() {
    echo -e "${GREEN}🏃 Running all critical path benchmarks...${NC}"
    cargo bench --bench critical_path
}

# Function to show available benchmarks
show_benchmarks() {
    echo -e "${BLUE}📋 Available Benchmarks:${NC}"
    echo "1. bench_atomic_metrics        - Lock-free metrics performance"
    echo "2. bench_stratum_parsing       - JSON message parsing"
    echo "3. bench_string_operations     - String interning and operations"
    echo "4. bench_hash_operations       - Hash functions and distribution"
    echo "5. bench_memory_operations     - Memory allocation patterns"
    echo "6. bench_json_operations       - JSON serialization/deserialization"
    echo "7. bench_message_validation    - Protocol message validation"
    echo "8. bench_concurrent_structures - Thread-safe data structures"
    echo "9. bench_time_operations       - Time and duration operations"
    echo "10. bench_protocol_methods     - Method routing and dispatch"
}

# Function to create benchmark baseline
create_baseline() {
    local baseline_name="${1:-main}"
    echo -e "${GREEN}📊 Creating benchmark baseline: ${baseline_name}${NC}"
    cargo bench --bench critical_path --save-baseline "${baseline_name}"
}

# Function to compare with baseline
compare_baseline() {
    local baseline_name="${1:-main}"
    echo -e "${BLUE}📈 Comparing with baseline: ${baseline_name}${NC}"
    cargo bench --bench critical_path --baseline "${baseline_name}"
}

# Function to show help
show_help() {
    echo -e "${GREEN}Usage: $0 [command] [arguments]${NC}"
    echo ""
    echo "Commands:"
    echo "  all                           - Run all benchmarks"
    echo "  list                          - Show available benchmarks"
    echo "  run <benchmark_name>          - Run specific benchmark"
    echo "  baseline [name]               - Create baseline (default: main)"
    echo "  compare [baseline_name]       - Compare with baseline (default: main)"
    echo "  help                          - Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 all                        # Run all benchmarks"
    echo "  $0 run bench_json_operations  # Run JSON benchmarks only"
    echo "  $0 baseline v1.0             # Create baseline named 'v1.0'"
    echo "  $0 compare v1.0              # Compare against 'v1.0' baseline"
    echo ""
    echo "Quick Performance Test:"
    echo "  timeout 30s $0 run bench_atomic_metrics"
}

# Function to run quick performance test
quick_test() {
    echo -e "${YELLOW}⚡ Running quick performance test (atomic metrics)...${NC}"
    timeout 30s cargo bench --bench critical_path bench_atomic_metrics || {
        echo -e "${RED}❌ Quick test timed out or failed${NC}"
        return 1
    }
    echo -e "${GREEN}✅ Quick test completed successfully${NC}"
}

# Main script logic
case "${1:-help}" in
    "all")
        run_all_benchmarks
        ;;
    "list")
        show_benchmarks
        ;;
    "run")
        if [ -z "$2" ]; then
            echo -e "${RED}❌ Error: benchmark name required${NC}"
            show_benchmarks
            exit 1
        fi
        run_benchmark "$2"
        ;;
    "baseline")
        create_baseline "$2"
        ;;
    "compare")
        compare_baseline "$2"
        ;;
    "quick")
        quick_test
        ;;
    "help"|*)
        show_help
        ;;
esac

echo -e "${GREEN}✨ Benchmark operation completed${NC}"