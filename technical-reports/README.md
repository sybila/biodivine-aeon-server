# Technical Reports

In this directory, we try to keep notes about technical analysis that 
informed the design of AEON. In particular, if an algorithm is to be 
included in AEON based on the merits of speed, there should be a technical 
report comparing it to an equivalent "naive" or older version of the same 
algorithm. Ideally, completely new algorithms should also include a 
technical report showing the potential speed that can be expected of the 
algorithm. 

**What's in a technical report?** Every report should contain at least 
two things: 

A readme file with a simple description of the problem, the algorithm, and 
the relevant benchmarks (including testing setup). The benchmark must show 
that the algorithm works across all relevant inputs and that it meaningfully 
improves on the naive/previous version. Also, *commit* and *compiler 
version* should be mentioned to aid reproducibility.

The second item is a reproducibility artefact. This is ideally a zip 
archive (to avoid spamming git) with all relevant benchmark inputs, outputs,
and binaries. The idea is that even if one cannot reproduce the results 
exactly, there should be at least a full record of what was tested and what 
was the output (in case somebody comes later and wants to audit the results).

Some other useful notes:
 - Please create the benchmark binaries in `examples/benchmarks`, adding the 
   relevant entry into `Cargo.toml`.
 - You can use the attached `run.py` to execute any native binary or python 
   script for a whole folder of input models, obtaining a useful statistic 
   of the result.
 - Ideally provide some basic amount of logging so that if the benchmark 
   fails, it can still be determined how far in the computation it got. 