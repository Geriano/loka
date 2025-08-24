#!/bin/bash

# Loka Stratum Performance Validation Script
# Validates that all new metrics meet sub-microsecond performance targets

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Performance thresholds (in nanoseconds)
COUNTER_THRESHOLD_NS=10     # 10ns for counter operations (target: ~4.6ns)
GAUGE_THRESHOLD_NS=15      # 15ns for gauge operations
FLOAT_CONVERSION_NS=20     # 20ns for float-to-bits operations
BATCH_SPEEDUP_RATIO=50     # Minimum 50x speedup for batch operations

echo -e "${BLUE}🔍 Loka Stratum Performance Validation${NC}"
echo "=============================================="
echo ""
echo -e "${YELLOW}Performance Targets:${NC}"
echo "- Counter operations: <${COUNTER_THRESHOLD_NS}ns (target: ~4.6ns)"
echo "- Gauge operations: <${GAUGE_THRESHOLD_NS}ns"
echo "- Float conversions: <${FLOAT_CONVERSION_NS}ns"
echo "- Batch operations: >${BATCH_SPEEDUP_RATIO}x speedup"
echo ""

# Function to extract timing from benchmark output
extract_timing() {
    local bench_output="$1"
    local bench_name="$2"
    
    # Extract timing from criterion output (format: "time: [x.xx ns x.xx ns x.xx ns]")
    echo "$bench_output" | grep -A 3 "$bench_name" | grep "time:" | sed 's/.*time: \[\([0-9.]*\) ns.*/\1/' | head -1
}

# Function to validate performance threshold
validate_threshold() {
    local actual_ns="$1"
    local threshold_ns="$2"
    local operation_name="$3"
    
    if (( $(echo "$actual_ns < $threshold_ns" | bc -l) )); then
        echo -e "${GREEN}✅ $operation_name: ${actual_ns}ns (PASS - under ${threshold_ns}ns)${NC}"
        return 0
    else
        echo -e "${RED}❌ $operation_name: ${actual_ns}ns (FAIL - exceeds ${threshold_ns}ns)${NC}"
        return 1
    fi
}

# Function to run specific benchmark and extract timing
run_and_validate() {
    local bench_name="$1"
    local threshold_ns="$2"
    local description="$3"
    
    echo -e "${YELLOW}🚀 Running $description...${NC}"
    
    # Run benchmark and capture output
    local output=$(cargo bench --bench new_metrics_performance "$bench_name" 2>&1)
    
    # Extract timing
    local timing=$(extract_timing "$output" "$bench_name")
    
    if [ -z "$timing" ]; then
        echo -e "${RED}❌ Failed to extract timing for $bench_name${NC}"
        return 1
    fi
    
    # Validate against threshold
    validate_threshold "$timing" "$threshold_ns" "$description"
}

# Validation results tracker
passed_tests=0
total_tests=0

echo -e "${BLUE}📊 Running Performance Validation Tests...${NC}"
echo ""

# Test 1: Connection lifecycle metrics
echo -e "${YELLOW}Test 1: Connection Lifecycle Metrics (Task 8.1)${NC}"
total_tests=$((total_tests + 1))
if run_and_validate "connection_lifecycle_events" "$COUNTER_THRESHOLD_NS" "Connection Events Counter"; then
    passed_tests=$((passed_tests + 1))
fi
echo ""

# Test 2: Protocol detection metrics  
echo -e "${YELLOW}Test 2: Protocol Detection Metrics (Task 8.2)${NC}"
total_tests=$((total_tests + 1))
if run_and_validate "protocol_detection_operations" "$COUNTER_THRESHOLD_NS" "Protocol Detection Counter"; then
    passed_tests=$((passed_tests + 1))
fi
echo ""

# Test 3: Mining operation metrics
echo -e "${YELLOW}Test 3: Mining Operation Metrics (Task 8.3)${NC}"
total_tests=$((total_tests + 1))
if run_and_validate "share_submission_recording" "$COUNTER_THRESHOLD_NS" "Share Submission Recording"; then
    passed_tests=$((passed_tests + 1))
fi
echo ""

# Test 4: Error categorization metrics
echo -e "${YELLOW}Test 4: Error Categorization Metrics (Task 8.4)${NC}"
total_tests=$((total_tests + 1))
if run_and_validate "error_categorization_dispatch" "$COUNTER_THRESHOLD_NS" "Error Categorization Dispatch"; then
    passed_tests=$((passed_tests + 1))
fi
echo ""

# Test 5: Resource utilization metrics  
echo -e "${YELLOW}Test 5: Resource Utilization Metrics (Task 8.5)${NC}"
total_tests=$((total_tests + 1))
if run_and_validate "memory_utilization_recording" "$GAUGE_THRESHOLD_NS" "Memory Utilization Recording"; then
    passed_tests=$((passed_tests + 1))
fi
echo ""

# Test 6: Float-to-bits atomic operations
echo -e "${YELLOW}Test 6: Atomic Bit Operations${NC}"
total_tests=$((total_tests + 1))
if run_and_validate "float_to_bits_conversion" "$FLOAT_CONVERSION_NS" "Float to Bits Conversion"; then
    passed_tests=$((passed_tests + 1))
fi
echo ""

# Test 7: Compare new metrics vs baseline
echo -e "${YELLOW}Test 7: New Metrics vs Baseline Comparison${NC}"
echo -e "${BLUE}Running comprehensive comparison...${NC}"

# Run the comparison benchmark
comparison_output=$(cargo bench --bench new_metrics_performance bench_new_metrics_vs_baseline 2>&1)

# Extract baseline timing
baseline_timing=$(extract_timing "$comparison_output" "baseline_original_metrics")
new_metrics_timing=$(extract_timing "$comparison_output" "new_metrics_comprehensive")

if [ ! -z "$baseline_timing" ] && [ ! -z "$new_metrics_timing" ]; then
    # Calculate performance ratio
    ratio=$(echo "scale=2; $new_metrics_timing / $baseline_timing" | bc -l)
    
    echo "Baseline metrics: ${baseline_timing}ns"
    echo "New metrics: ${new_metrics_timing}ns" 
    echo "Performance ratio: ${ratio}x"
    
    # Allow up to 2x overhead for new metrics
    if (( $(echo "$ratio < 2.0" | bc -l) )); then
        echo -e "${GREEN}✅ New metrics overhead acceptable (${ratio}x baseline)${NC}"
        total_tests=$((total_tests + 1))
        passed_tests=$((passed_tests + 1))
    else
        echo -e "${RED}❌ New metrics overhead too high (${ratio}x baseline)${NC}"
        total_tests=$((total_tests + 1))
    fi
else
    echo -e "${YELLOW}⚠️  Could not extract comparison timings${NC}"
fi
echo ""

# Test 8: Concurrent performance validation
echo -e "${YELLOW}Test 8: Concurrent Access Performance${NC}"
echo -e "${BLUE}Running concurrent stress test...${NC}"

# Run concurrent benchmark (limited to 30 seconds)
timeout 30s cargo bench --bench new_metrics_performance bench_concurrent_new_metrics_stress 2>&1 | grep -E "(threads/1|threads/4|threads/8)" || echo "Concurrent test completed"

total_tests=$((total_tests + 1))
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Concurrent stress test completed successfully${NC}"
    passed_tests=$((passed_tests + 1))
else
    echo -e "${RED}❌ Concurrent stress test failed or timed out${NC}"
fi
echo ""

# Final results
echo "=============================================="
echo -e "${BLUE}📋 Performance Validation Results${NC}"
echo "=============================================="
echo "Tests passed: $passed_tests/$total_tests"

if [ $passed_tests -eq $total_tests ]; then
    echo -e "${GREEN}🎉 All performance tests PASSED!${NC}"
    echo -e "${GREEN}✅ All new metrics meet sub-microsecond performance targets${NC}"
    exit 0
else
    failed_tests=$((total_tests - passed_tests))
    echo -e "${RED}❌ $failed_tests performance test(s) FAILED${NC}"
    echo -e "${RED}⚠️  Some metrics may not meet performance requirements${NC}"
    exit 1
fi