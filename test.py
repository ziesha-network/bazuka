#!/usr/bin/python3
import subprocess

NUM_NODES = 4

procs = [
    subprocess.Popen([
        './target/debug/bazuka',
        '--db', 'nodes/node{}'.format(i),
        '--listen', "127.0.0.1:" + str(3030 + i),
        '--external', "127.0.0.1:" + str(3030 + i),
        '--bootstrap', "127.0.0.1:3030"
    ], stdout=subprocess.PIPE)
    for i in range(NUM_NODES)
]

while True:
    for i, p in enumerate(procs):
        state = p.stdout.readline().decode('utf-8')
        print(i, ':', state)
