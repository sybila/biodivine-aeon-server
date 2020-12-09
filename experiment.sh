# Set memory limit to 16GB
ulimit -v 16000000
# Set time limit to 1 hour and run experiment
timeout 1h nice -n 19 ./target/release/experiment < $1
retVal=$?
echo "Status: " $retVal
if [ $retVal -eq 124 ]; then
    echo "Timeout."
elif [ $retVal -eq 134 ]; then
    echo "Memory exhausted."
else
    echo "Success."
fi

