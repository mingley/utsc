"""Linux-only child-process measurement. JSON request on stdin, result on stdout."""
import json
import os
import resource
import signal
import subprocess
import sys
import time


def main():
    request = json.load(sys.stdin)
    start = time.perf_counter_ns()
    process = subprocess.Popen(
        request['command'], cwd=request['cwd'],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True,
    )
    timed_out = False
    try:
        stdout, stderr = process.communicate(timeout=request['timeout_seconds'])
    except subprocess.TimeoutExpired:
        timed_out = True
        os.killpg(process.pid, signal.SIGKILL)
        stdout, stderr = process.communicate()
    elapsed = (time.perf_counter_ns() - start) / 1_000_000
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    json.dump({
        'exit_code': process.returncode,
        'timed_out': timed_out,
        'wall_ms': elapsed,
        'cpu_ms': (usage.ru_utime + usage.ru_stime) * 1000,
        # Linux reports KiB, and this isolated wrapper measures one invocation.
        # This is the largest child RSS, not a concurrent process-tree sum.
        'max_rss_kib': usage.ru_maxrss,
        'stdout': stdout.decode('utf-8', errors='strict'),
        'stderr': stderr.decode('utf-8', errors='strict'),
    }, sys.stdout)


if __name__ == '__main__':
    main()
