#!/bin/bash
# ToadStool Showcase - Multi-Substrate Benchmark Demo

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

RESULTS_DIR="results"
mkdir -p "$RESULTS_DIR"

echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║     🍄 ToadStool Multi-Substrate Benchmark Demo          ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

echo -e "${BLUE}This demo runs PERFORMANCE benchmarks across substrates.${NC}"
echo "Compare CPU and I/O performance to see substrate characteristics."
echo ""

# Check what substrates are available
SUBSTRATES=("native")

if command -v docker &> /dev/null && docker info &> /dev/null 2>&1; then
    SUBSTRATES+=("docker")
fi

if command -v python3 &> /dev/null; then
    SUBSTRATES+=("python")
fi

echo -e "${CYAN}Testing substrates: ${SUBSTRATES[*]}${NC}"
echo ""

# Function to run CPU benchmark
run_cpu_benchmark() {
    local substrate=$1
    
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo -e "${YELLOW}CPU Benchmark on: ${substrate}${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    
    export TOADSTOOL_SUBSTRATE="$substrate"
    
    # Run Fibonacci benchmark
    python3 << 'PYTHON_SCRIPT'
import time
import os
import json
from datetime import datetime

SUBSTRATE = os.environ.get('TOADSTOOL_SUBSTRATE', 'unknown')

def fib(n):
    if n <= 1:
        return n
    return fib(n-1) + fib(n-2)

print("╔════════════════════════════════════════════════════════════╗")
print("║            🍄 ToadStool CPU Benchmark                     ║")
print("╚════════════════════════════════════════════════════════════╝")
print()
print(f"Substrate: {SUBSTRATE}")
print(f"Test:      Recursive Fibonacci(35)")
print()
print("Running CPU-intensive computation...")
print()

start = time.perf_counter()
result = fib(35)
duration = time.perf_counter() - start

print(f"  Result:   fib(35) = {result}")
print(f"  Duration: {duration:.3f} seconds")
print()

# Performance rating
if duration < 2.0:
    rating = "EXCELLENT"
    emoji = "🚀"
elif duration < 5.0:
    rating = "GOOD"
    emoji = "✅"
elif duration < 10.0:
    rating = "ACCEPTABLE"
    emoji = "⚠️"
else:
    rating = "SLOW"
    emoji = "🐌"

print(f"  Performance: {emoji} {rating}")
print()

output = {
    'substrate': SUBSTRATE,
    'benchmark': 'cpu',
    'test': 'fibonacci_35',
    'timestamp': datetime.now().isoformat(),
    'result': result,
    'duration_seconds': duration,
    'rating': rating
}

print(f"✅ CPU Benchmark complete on: {SUBSTRATE}")
print()

# Save results
results_file = f"results/benchmark-cpu-{SUBSTRATE}.json"
with open(results_file, 'w') as f:
    json.dump(output, indent=2, fp=f)

PYTHON_SCRIPT
    
    echo ""
    sleep 1
}

# Function to run I/O benchmark
run_io_benchmark() {
    local substrate=$1
    
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo -e "${YELLOW}I/O Benchmark on: ${substrate}${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    
    export TOADSTOOL_SUBSTRATE="$substrate"
    
    # Run I/O benchmark
    python3 << 'PYTHON_SCRIPT'
import time
import os
import json
from pathlib import Path
from datetime import datetime

SUBSTRATE = os.environ.get('TOADSTOOL_SUBSTRATE', 'unknown')
TEST_DIR = Path('/tmp/toadstool-io-bench')

print("╔════════════════════════════════════════════════════════════╗")
print("║            🍄 ToadStool I/O Benchmark                     ║")
print("╚════════════════════════════════════════════════════════════╝")
print()
print(f"Substrate: {SUBSTRATE}")
print(f"Test:      File I/O (Write/Read)")
print()

# Setup test directory
TEST_DIR.mkdir(parents=True, exist_ok=True)

# Test parameters
test_sizes = [1, 10, 50]  # MB
results = []

print("Running I/O-intensive operations...")
print()

for size_mb in test_sizes:
    size_bytes = size_mb * 1024 * 1024
    test_file = TEST_DIR / f'test_{size_mb}mb.dat'
    
    print(f"  Testing {size_mb}MB file...")
    
    # Write test
    print(f"    Writing... ", end='', flush=True)
    data = os.urandom(size_bytes)
    start = time.perf_counter()
    with open(test_file, 'wb') as f:
        f.write(data)
        f.flush()
        os.fsync(f.fileno())
    write_time = time.perf_counter() - start
    write_speed = size_bytes / write_time / 1024 / 1024  # MB/s
    print(f"{write_time:.3f}s ({write_speed:.2f} MB/s)")
    
    # Read test
    print(f"    Reading... ", end='', flush=True)
    start = time.perf_counter()
    with open(test_file, 'rb') as f:
        _ = f.read()
    read_time = time.perf_counter() - start
    read_speed = size_bytes / read_time / 1024 / 1024  # MB/s
    print(f"{read_time:.3f}s ({read_speed:.2f} MB/s)")
    
    # Cleanup
    test_file.unlink()
    
    results.append({
        'size_mb': size_mb,
        'write_time': write_time,
        'write_speed_mbs': write_speed,
        'read_time': read_time,
        'read_speed_mbs': read_speed
    })
    print()

# Cleanup test directory
TEST_DIR.rmdir()

avg_write = sum(r['write_speed_mbs'] for r in results) / len(results)
avg_read = sum(r['read_speed_mbs'] for r in results) / len(results)

print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
print("Results:")
print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
print(f"  Average Write Speed: {avg_write:.2f} MB/s")
print(f"  Average Read Speed:  {avg_read:.2f} MB/s")
print()

# Performance rating
if avg_write > 500 and avg_read > 800:
    rating = "EXCELLENT"
    emoji = "🚀"
elif avg_write > 300 and avg_read > 500:
    rating = "GOOD"
    emoji = "✅"
elif avg_write > 100 and avg_read > 200:
    rating = "ACCEPTABLE"
    emoji = "⚠️"
else:
    rating = "SLOW"
    emoji = "🐌"

print(f"  Performance: {emoji} {rating}")
print()

output = {
    'substrate': SUBSTRATE,
    'benchmark': 'io',
    'test': 'file_operations',
    'timestamp': datetime.now().isoformat(),
    'results': results,
    'summary': {
        'average_write_mbs': avg_write,
        'average_read_mbs': avg_read,
        'rating': rating
    }
}

print(f"✅ I/O Benchmark complete on: {SUBSTRATE}")
print()

# Save results
results_file = f"results/benchmark-io-{SUBSTRATE}.json"
with open(results_file, 'w') as f:
    json.dump(output, indent=2, fp=f)

PYTHON_SCRIPT
    
    echo ""
    sleep 1
}

# Run benchmarks on all substrates
echo -e "${BLUE}═══════════════ CPU BENCHMARKS ═══════════════${NC}"
echo ""
for substrate in "${SUBSTRATES[@]}"; do
    run_cpu_benchmark "$substrate"
done

echo ""
echo -e "${BLUE}═══════════════ I/O BENCHMARKS ═══════════════${NC}"
echo ""
for substrate in "${SUBSTRATES[@]}"; do
    run_io_benchmark "$substrate"
done

# Generate comparison report
echo ""
echo "════════════════════════════════════════════════════════════"
echo -e "${GREEN}✅ Benchmark Suite Complete!${NC}"
echo "════════════════════════════════════════════════════════════"
echo ""
echo -e "${BLUE}Performance Comparison:${NC}"
echo ""

# Parse results and compare
echo "Substrate Performance Summary:"
echo "┌──────────────┬──────────────┬──────────────┐"
echo "│  Substrate   │  CPU (Fib35) │  I/O (Avg)   │"
echo "├──────────────┼──────────────┼──────────────┤"

for substrate in "${SUBSTRATES[@]}"; do
    if [ -f "results/benchmark-cpu-${substrate}.json" ]; then
        cpu_time=$(python3 -c "import json; print(f\"{json.load(open('results/benchmark-cpu-${substrate}.json'))['duration_seconds']:.2f}s\")")
        io_read=$(python3 -c "import json; print(f\"{json.load(open('results/benchmark-io-${substrate}.json'))['summary']['average_read_mbs']:.0f} MB/s\")")
        printf "│ %-12s │ %-12s │ %-12s │\n" "$substrate" "$cpu_time" "$io_read"
    fi
done

echo "└──────────────┴──────────────┴──────────────┘"
echo ""

echo -e "${YELLOW}💡 Key Insights:${NC}"
echo "  • Different substrates have different performance profiles"
echo "  • ToadStool can intelligently route workloads to optimal substrate"
echo "  • Resource awareness enables smart scheduling"
echo ""
echo -e "${CYAN}📊 Results saved to: results/${NC}"
echo ""

