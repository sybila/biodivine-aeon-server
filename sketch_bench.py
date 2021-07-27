import sys
import os
import re
import time
from shlex import quote

RE_TIME = re.compile("\\s*real\\s*(\\d+\\.?\\d*)\\s*")

print(">>>>>>>>>> COMPILE RUST BINARIES")

os.system("cargo build --release")

print(">>>>>>>>>> START BENCHMARK RUN")

BENCH_DIR = sys.argv[1]
print("Benchmark directory:", BENCH_DIR)

# Set binary based on algorithm (also setup so that an input file can be appended there).
BINARY = "./target/release/sketch_synthesis"

# Utility function to check if a given path is a benchmark model.
def is_bench(benchmark):
	return benchmark.endswith(".aeon")

# Create output directory
OUT_DIR = BENCH_DIR + "_run_" + str(int(time.time()))
os.mkdir(OUT_DIR)

BENCHMARKS = filter(is_bench, os.listdir(BENCH_DIR))
BENCHMARKS = sorted(BENCHMARKS)

for bench in BENCHMARKS:
	print(">>>>>>>>>> START MODEL", bench)
	# Filename without extension
	name = os.path.splitext(bench)[0]
	input_file = BENCH_DIR + "/" + bench
	output_file = OUT_DIR + "/" + name + "_out.txt"
	command = BINARY + " < " + input_file + " > " + output_file + " 2>&1"
	print(command)
	os.system(command)