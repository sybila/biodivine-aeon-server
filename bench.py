import os
import re
import time

re_bench = re.compile("\[v(\d+)\]__\[r(\d+)\]__\[(.+?)\]__\[(.+?)\]")
re_var_count = re.compile("\[v(\d+)\]")

def is_bench(benchmark):
	return re_bench.match(benchmark) != None

def bench_cmp(benchmark):
	m = re_var_count.match(benchmark)
	return int(m.group(1))

benchmarks = filter(is_bench, os.listdir("./benchmark"))
benchmarks = sorted(benchmarks, key=bench_cmp)


out_dir = "bench" + str(int(time.time()))
os.mkdir(out_dir)

i = 1
for benchmark in benchmarks:	
	bench_name = re_bench.match(benchmark).group(3)
	print("Starting "+bench_name)
	in_file = "./benchmark/" + benchmark + "/model.aeon"
	out_file = "./" + out_dir + "/" + str(i) + "_" + bench_name + ".txt"
	os.system("./target/release/experiment < " + in_file + " > " + out_file)
	i = i + 1
	with open(out_file, 'r') as f:
		lines = f.read().splitlines()
		time_line = lines[-1]
		class_line = lines[-3]
		print(class_line + " " + time_line)