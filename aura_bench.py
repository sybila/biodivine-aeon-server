import os
import re
import time

re_bench = re.compile("\[v(\d+)\]__\[r(\d+)\]__\[(.+?)\]__\[(.+?)\]")
re_var_count = re.compile("\[v(\d+)\]")
re_elapsed = re.compile("Elapsed time: (\d+\.?\d*)s")

def is_bench(benchmark):
	return re_bench.match(benchmark) != None

def bench_cmp(benchmark):
	m = re_var_count.match(benchmark)
	return int(m.group(1))

benchmarks = filter(is_bench, os.listdir("./benchmark"))
benchmarks = sorted(benchmarks, key=bench_cmp)


out_dir = "bench_run_" + str(int(time.time()))
os.mkdir(out_dir)

elapsed_times = {}
i = 1
for benchmark in benchmarks:
	bench_name = re_bench.match(benchmark).group(3)
	print("Starting "+bench_name)
	in_file = "./benchmark/" + benchmark + "/model.aeon"
	out_file = "./" + out_dir + "/" + str(i) + "_" + bench_name + ".txt"
	os.system("./experiment.sh " + in_file + " > " + out_file)
	i = i + 1
	with open(out_file, 'r') as f:
		lines = f.read().splitlines()
		status = lines[-1]
		if status == "Success.":
			time_line = lines[-3]
			class_line = lines[-5]
			print(class_line + " " + time_line)
			time = re_elapsed.match(time_line).group(1)
			elapsed_times[bench_name] = time
		else:
			elapsed_times[bench_name] = status

print "FINISHED"
print "Benchmark, Time[s]"
for bench, time in  elapsed_times.items():
	print bench + ", " + str(time)

# Write the same results to a file:
f = open(out_dir + "/stats.csv", "w")
f.write("Benchmark, Time[s]\n")
for bench, time in  elapsed_times.items():
        f.write(bench + ", " + str(time) + "\n")
f.close()
