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
    
    # Determine which benchmark file to run based on benchmark name
    if [[ "${bench_name}" =~ ^(connection_lifecycle|protocol_detection|mining_operation|error_categorization|resource_utilization|new_metrics_vs_baseline|atomic_bit_operations|concurrent_new_metrics_stress) ]]; then
        cargo bench --bench new_metrics_performance "${bench_name}"
    else
        cargo bench --bench critical_path "${bench_name}"
    fi
}

# Function to run all benchmarks
run_all_benchmarks() {
    echo -e "${GREEN}🏃 Running all critical path benchmarks...${NC}"
    echo -e "${BLUE}📊 Running original critical path benchmarks...${NC}"
    cargo bench --bench critical_path
    echo -e "${BLUE}📈 Running new metrics performance benchmarks...${NC}"
    cargo bench --bench new_metrics_performance
}

# Function to show available benchmarks
show_benchmarks() {
    echo -e "${BLUE}📋 Available Benchmarks:${NC}"
    echo ""
    echo -e "${GREEN}Original Critical Path Benchmarks:${NC}"
    echo "1.  bench_atomic_metrics        - Lock-free metrics performance"
    echo "2.  bench_stratum_parsing       - JSON message parsing"
    echo "3.  bench_string_operations     - String interning and operations"
    echo "4.  bench_hash_operations       - Hash functions and distribution"
    echo "5.  bench_memory_operations     - Memory allocation patterns"
    echo "6.  bench_json_operations       - JSON serialization/deserialization"
    echo "7.  bench_message_validation    - Protocol message validation"
    echo "8.  bench_concurrent_structures - Thread-safe data structures"
    echo "9.  bench_time_operations       - Time and duration operations"
    echo "10. bench_protocol_methods      - Method routing and dispatch"
    echo ""
    echo -e "${YELLOW}New Metrics Performance Benchmarks:${NC}"
    echo "11. bench_connection_lifecycle_metrics     - Connection duration, idle time, reconnects (Task 8.1)"
    echo "12. bench_protocol_detection_metrics       - HTTP vs Stratum detection performance (Task 8.2)"
    echo "13. bench_mining_operation_metrics         - Share submissions, job distribution (Task 8.3)"
    echo "14. bench_error_categorization_metrics     - Error classification and counting (Task 8.4)"
    echo "15. bench_resource_utilization_metrics     - Memory and CPU tracking (Task 8.5)"
    echo "16. bench_new_metrics_vs_baseline          - Performance comparison against baseline"
    echo "17. bench_atomic_bit_operations            - Float-to-bits atomic operations"
    echo "18. bench_concurrent_new_metrics_stress    - Concurrent access stress test"
}

# Function to create benchmark baseline
create_baseline() {
    local baseline_name="${1:-main}"
    echo -e "${GREEN}📊 Creating benchmark baseline: ${baseline_name}${NC}"
    echo -e "${BLUE}Creating baseline for critical path benchmarks...${NC}"
    cargo bench --bench critical_path --save-baseline "${baseline_name}"
    echo -e "${BLUE}Creating baseline for new metrics benchmarks...${NC}"
    cargo bench --bench new_metrics_performance --save-baseline "${baseline_name}"
}

# Function to compare with baseline
compare_baseline() {
    local baseline_name="${1:-main}"
    echo -e "${BLUE}📈 Comparing with baseline: ${baseline_name}${NC}"
    echo -e "${BLUE}Comparing critical path benchmarks with baseline...${NC}"
    cargo bench --bench critical_path --baseline "${baseline_name}"
    echo -e "${BLUE}Comparing new metrics benchmarks with baseline...${NC}"
    cargo bench --bench new_metrics_performance --baseline "${baseline_name}"
}

# Function to show help
show_help() {
    echo -e "${GREEN}Usage: $0 [command] [arguments]${NC}"
    echo ""
    echo "Commands:"
    echo "  all                           - Run all benchmarks (critical path + new metrics)"
    echo "  list                          - Show available benchmarks"
    echo "  run <benchmark_name>          - Run specific benchmark"
    echo "  baseline [name]               - Create baseline for all benchmarks (default: main)"
    echo "  compare [baseline_name]       - Compare with baseline for all benchmarks (default: main)"
    echo "  quick                         - Quick test of original atomic metrics (~30s)"
    echo "  quick-new                     - Quick test of new metrics performance (~30s)"
    echo "  quick-all                     - Quick test of both original and new metrics (~60s)"
    echo "  help                          - Show this help"
    echo ""
    echo "Examples:"
    echo "  $0 all                                    # Run all benchmarks"
    echo "  $0 run bench_json_operations              # Run JSON benchmarks only"
    echo "  $0 run bench_connection_lifecycle_metrics # Run new connection metrics"
    echo "  $0 baseline v1.0                         # Create baseline named 'v1.0'"
    echo "  $0 compare v1.0                          # Compare against 'v1.0' baseline"
    echo "  $0 quick-all                             # Quick performance test for all metrics"
    echo ""
    echo "Performance Targets:"
    echo "  Counter operations: ~4.6ns (sub-microsecond)"
    echo "  All new metrics: <1μs per operation"
    echo "  Batch operations: 100x+ speedup over individual"
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

# Function to run quick performance test for new metrics
quick_test_new() {
    echo -e "${YELLOW}⚡ Running quick performance test for new metrics...${NC}"
    timeout 30s cargo bench --bench new_metrics_performance bench_new_metrics_vs_baseline || {
        echo -e "${RED}❌ New metrics quick test timed out or failed${NC}"
        return 1
    }
    echo -e "${GREEN}✅ New metrics quick test completed successfully${NC}"
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
    "quick-new")
        quick_test_new
        ;;
    "quick-all")
        quick_test && quick_test_new
        ;;
    "help"|*)
        show_help
        ;;
esac

echo -e "${GREEN}✨ Benchmark operation completed${NC}"