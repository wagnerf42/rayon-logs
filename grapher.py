#!/usr/bin/env python3

import sys
import json
from collections import defaultdict
from functools import reduce
from itertools import chain
from more_itertools import windowed, roundrobin

COLORS = ["red", "green", "blue"]

class Task:
    def __init__(self, start_time, end_time, thread):
        self.start_time = start_time
        self.end_time = end_time
        self.thread = thread
        self.width = end_time - start_time
        self.height = 1
        self.position = [0, 0]
    def dimensions(self):
        return [self.width, self.height]
    def compute_positions(self):
        pass
    def svg(self, scales):
        """
        print svg code for this task's display
        """
        color = COLORS[self.thread % len(COLORS)]
        print("<rect width='{}' height='{}' x='{}' y='{}' style='fill:{}'/>".format(self.width*scales[0], scales[1], self.position[0] * scales[0], self.position[1] * scales[1], color))

class SubGraph:
    def __init__(self, content, is_parallel):
        self.content = content
        self.is_parallel = is_parallel
        if self.is_parallel:
            reducer = lambda d1, d2: (d1[0]+d2[0], max(d1[1], d2[1]))
        else:
            reducer = lambda d1, d2: (max(d1[0], d2[0]), d1[1]+d2[1])
        self.size = reduce(reducer, (c.dimensions() for c in self.content))
        self.position = [0, 0]

    def dimensions(self):
        return self.size

    def svg(self, scales):
        """
        print svg code
        """
        for subgraph in self.content:
            subgraph.svg(scales)

    def compute_positions(self):
        """
        compute x and y coordinates for each subgraph
        """
        pos = list(self.position)
        size = self.dimensions()
        for subgraph in self.content:
            subsize = subgraph.dimensions()
            if self.is_parallel:
                pos[1] = self.position[1] + (size[1]-subsize[1])/2
            else:
                pos[0] = self.position[0] + (size[0]-subsize[0])/2
            subgraph.position = list(pos)
            if self.is_parallel:
                pos[0] += subsize[0]
            else:
                pos[1] += subsize[1]
            subgraph.compute_positions()


def build_graph(root, children, spans):
    if children[root]:
        subgraphs = (build_graph(c, children, spans) for c in children[root])
        if spans[root]["tag"] == "join":
            content = subgraphs
            is_parallel = True
        else:
            # we need to loop on all in between periods
            times = ((spans[c]["start"], spans[c]["end"])
                      for c in children[root])
            all_times = chain([(None, spans[root]["start"])],
                              times,
                              [(spans[root]["end"], None)],
                        )
            consecutives = windowed(all_times, 2)
            intervals = ((c[0][1], c[1][0]) for c in consecutives)
            thread = spans[root]["thread"]
            tasks = (Task(start, end, thread) for start, end in intervals)
            content = roundrobin(tasks, subgraphs)
            is_parallel = False
        return SubGraph(list(content), is_parallel)
    else:
        # if we have no children we are a final task
        return Task(
                    spans[root]["start"],
                    spans[root]["end"],
                    spans[root]["thread"],
                )


def main():
    """
    convert json log file to svg.
    """
    if len(sys.argv) != 2:
        print("we need a json log filename")
        sys.exit(1)
    with open(sys.argv[1]) as json_file:
        spans = json.load(json_file)

    # TODO: we should check there is no 0 and no duplicates
    spans = dict((s["ID"], s) for s in spans)

    children = defaultdict(list)
    roots = []
    for span_id, span in spans.items():
        if span["parent"] != 0:
            children[span["parent"]].append(span_id)
        else:
            roots.append(span_id)
    assert len(roots) == 1
    graph = build_graph(roots[0], children, spans)
    graph.compute_positions()
    print('<svg version="1.1" viewBox="0 0 1920 1080" xmlns="http://www.w3.org/2000/svg">')
    size = graph.dimensions()
    scales = (1920/size[0], 1080/size[1])
    graph.svg(scales)
    print('</svg>')


main()
