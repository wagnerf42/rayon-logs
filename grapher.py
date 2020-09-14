#!/usr/bin/env python3

import sys
import json
from collections import defaultdict
from functools import reduce
from itertools import chain, product
from more_itertools import windowed, roundrobin

COLORS = ["red", "green", "blue"]


class Task:
    """
    One Task is not a rayon task but one box to animate
    """

    def __init__(self, start_time, end_time, thread):
        self.start_time = start_time
        self.end_time = end_time
        self.thread = thread
        self.width = end_time - start_time
        self.height = 1
        self.position = [0, 0]

    def dimensions(self):
        """
        return width and height needed for display
        """
        return [self.width, self.height]

    def compute_positions(self):
        """
        this is an empty function to stop recursion from SubGraph
        """
        pass

    def svg(self, scales):
        """
        print svg code for this task's display
        """
        color = COLORS[self.thread % len(COLORS)]
        print("<rect width='{}' height='{}' \
x='{}' y='{}' style='fill:{}'\
/>".format(self.width*scales[0], scales[1]/2,
           self.position[0] * scales[0],
           # let's skip some height to leave space for edges
           (self.position[1]+0.25) * scales[1], color))

    def entry_points(self, scales):
        """
        return (scaled) coordinates of entry point
        (for edges)
        """
        return [((self.position[0]+self.width/2)*scales[0],
                 (self.position[1]+0.25) * scales[1])]

    def exit_points(self, scales):
        """
        return (scaled) coordinates of entry point
        (for edges)
        """
        return [((self.position[0]+self.width/2)*scales[0],
                 (self.position[1]+0.75) * scales[1])]

    def draw_edges(self, entry_points, exit_points, scales):
        pass


class SubGraph:
    """
    one node in a tree constituing the graph.
    contains either a sequence of subgraphs
    or parallel subgraphs
    """

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
        """
        return width and height
        """
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

    def entry_points(self, scales):
        if self.is_parallel:
            return [x for c in self.content for x in c.entry_points(scales)]
        return self.content[0].entry_points(scales)

    def exit_points(self, scales):
        if self.is_parallel:
            return [x for c in self.content for x in c.exit_points(scales)]
        return self.content[-1].exit_points(scales)

    def draw_edges(self, entry_points, exit_points, scales):
        if self.is_parallel:
            for subgraph in self.content:
                subgraph.draw_edges(entry_points, exit_points, scales)
        else:
            for start, end in product(entry_points, self.content[0].entry_points(scales)):
                edge(start, end)
            for start, end in product(exit_points, self.content[-1].exit_points(scales)):
                edge(start, end)
            for s1, s2, s3 in windowed(self.content, 3):
                if s3 is not None:
                    s2.draw_edges(s1.exit_points(scales), s3.entry_points(scales), scales)


def edge(start, end):
    """
    draw svg segment
    """
    print("<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='black' stroke-width='3'/>".format(*start, *end))


def build_graph(root, children, spans):
    """
    build a tree corresponding to the task graph rooted at given root.
    positions are not computed as they need a second pass
    """
    subgraphs = (build_graph(c, children, spans) for c in children[root])
    if spans[root]["tag"] == "join":
        # in between periods are hidden. they should appear in idle gantt
        content = subgraphs
        is_parallel = True
    else:
        # we need to loop on all "in between" periods
        times = ((spans[c]["start"], spans[c]["end"])
                 for c in children[root])
        all_times = chain([(None, spans[root]["start"])],
                          times,
                          [(spans[root]["end"], None)])
        consecutives = windowed(all_times, 2)
        intervals = ((c[0][1], c[1][0]) for c in consecutives)
        thread = spans[root]["thread"]
        tasks = (Task(start, end, thread) for start, end in intervals)
        content = roundrobin(tasks, subgraphs)
        is_parallel = False
    return SubGraph(list(content), is_parallel)


def load_spans(filename):
    """
    load json file containing spans and indexed them by id
    """
    with open(filename) as json_file:
        spans = json.load(json_file)

    indexed_spans = dict()
    for span in spans:
        identifier = span["ID"]
        assert identifier != 0  # 0 is reserved for no parent
        assert identifier not in indexed_spans  # all are unique
        indexed_spans[identifier] = span
    return indexed_spans


class Graph:
    """
    fork-join task graph as displayed in rayon-logs
    """

    def __init__(self, spans):
        # compute children and root spans
        children = defaultdict(list)
        roots = []
        for span_id, span in spans.items():
            if span["parent"] != 0:
                children[span["parent"]].append(span_id)
            else:
                roots.append(span_id)

        # for now, we only work with one root
        assert len(roots) == 1

        self.root = build_graph(roots[0], children, spans)
        self.root.compute_positions()

    def svg(self):
        """
        print svg code
        """
        print('<svg version="1.1" viewBox="0 0 1920 1080" \
    xmlns="http://www.w3.org/2000/svg">')
        size = self.root.dimensions()
        scales = (1920/size[0], 1080/size[1])
        self.root.svg(scales)
        self.root.draw_edges([], [], scales)
        print('</svg>')


def main():
    """
    convert json log file to svg.
    """
    if len(sys.argv) != 2:
        print("we need a json log filename")
        sys.exit(1)

    spans = load_spans(sys.argv[1])
    graph = Graph(spans)
    graph.svg()


main()
