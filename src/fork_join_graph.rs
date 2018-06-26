//! Store a trace as a fork join graph (in a vector).
use {svg::COLORS, Rectangle, TaskId, TaskLog};
type BlockId = usize;
use std::collections::HashMap;

const VERTICAL_GAP: f64 = 0.2;

use svg::Point;

trait BlockVector {
    fn push_block(&mut self, block: Block) -> BlockId;
    fn sequence(&mut self, id: BlockId) -> &mut Vec<BlockId>;
    fn parallel(&mut self, id: BlockId) -> &mut Vec<BlockId>;
    fn add_task(&mut self, task: TaskLog) -> BlockId;
    fn add_sequence(&mut self) -> BlockId;
    fn add_parallel(&mut self) -> BlockId;
}

impl BlockVector for Vec<Block> {
    fn add_task(&mut self, task: TaskLog) -> BlockId {
        self.push_block(Block::Task(task))
    }

    fn add_sequence(&mut self) -> BlockId {
        self.push_block(Block::Sequence(Vec::new()))
    }

    fn add_parallel(&mut self) -> BlockId {
        self.push_block(Block::Parallel(Vec::new()))
    }

    fn push_block(&mut self, block: Block) -> BlockId {
        self.push(block);
        self.len() - 1
    }
    fn sequence(&mut self, id: BlockId) -> &mut Vec<BlockId> {
        match self[id] {
            Block::Sequence(ref mut s) => s,
            _ => panic!("should be a sequence"),
        }
    }
    fn parallel(&mut self, id: BlockId) -> &mut Vec<BlockId> {
        match self[id] {
            Block::Parallel(ref mut s) => s,
            _ => panic!("should be a parallel block"),
        }
    }
}

#[derive(Debug)]
enum Block {
    Task(TaskLog),
    Sequence(Vec<BlockId>),
    Parallel(Vec<BlockId>),
}

/// Create a fork join graph (stored in a vec). This is used to convert the logs into
/// a graphical display of animated rectangles.
fn create_graph(tasks: &[TaskLog]) -> Vec<Block> {
    let mut graph = vec![Block::Sequence(Vec::new())];

    // ok let's start now
    let mut fathers: HashMap<BlockId, BlockId> = HashMap::new();
    let mut current_blocks: HashMap<TaskId, BlockId> = HashMap::new();
    current_blocks.insert(0, 0); // init task and all its descendants go in initial sequence

    // we sort by starting time to be sure fathers are processed before children
    let mut sorted_tasks: Vec<TaskId> = (0..tasks.len()).collect();
    sorted_tasks.sort_unstable_by(|t1, t2| {
        tasks[*t1]
            .start_time
            .partial_cmp(&tasks[*t2].start_time)
            .unwrap()
    });

    for task_id in &sorted_tasks {
        let task = &tasks[*task_id];
        let current_block = current_blocks[task_id];

        // add task to its sequence
        let new_block = graph.add_task((*task).clone());
        graph.sequence(current_blocks[task_id]).push(new_block);

        // now look at the children
        if task.children.len() == 1 {
            let child = task.children[0];
            let possible_existing_block = current_blocks.remove(&child);
            if let Some(existing_block) = possible_existing_block {
                // hard case, this child has several fathers.
                current_blocks.insert(child, fathers[&current_block]);
                assert_eq!(fathers[&current_block], fathers[&existing_block]);
            } else {
                //easy case, first time child is seen by a father. maybe he has only one.
                current_blocks.insert(child, current_block);
            }
        } else if !task.children.is_empty() {
            let parallel_block = graph.add_parallel();
            graph.sequence(current_blocks[task_id]).push(parallel_block);
            for child in &task.children {
                let sequential_block = graph.add_sequence();
                graph.parallel(parallel_block).push(sequential_block);
                fathers.insert(sequential_block, current_block);
                let should_be_none = current_blocks.insert(*child, sequential_block);
                assert!(should_be_none.is_none());
            }
        }
    }
    graph
}

/// recursively compute widths and heights of block at given index and all its sub-blocks.
fn compute_blocks_dimensions(
    index: BlockId,
    graph: &[Block],
    blocks_dimensions: &mut [(f64, f64)],
) -> (f64, f64) {
    let dimensions = match graph[index] {
        Block::Sequence(ref s) => s.iter().fold((0.0, -VERTICAL_GAP), |dimensions, id| {
            let (width, height) = compute_blocks_dimensions(*id, &graph, blocks_dimensions);
            (
                if width > dimensions.0 {
                    width
                } else {
                    dimensions.0
                },
                height + dimensions.1 + VERTICAL_GAP,
            )
        }),
        Block::Parallel(ref p) => p.iter().fold((0.0, 0.0), |dimensions, id| {
            let (width, height) = compute_blocks_dimensions(*id, &graph, blocks_dimensions);
            (
                width + dimensions.0,
                if height > dimensions.1 {
                    height
                } else {
                    dimensions.1
                },
            )
        }),
        Block::Task(ref t) => ((t.end_time - t.start_time) as f64, 1.0),
    };
    blocks_dimensions[index] = dimensions;
    dimensions
}

/// Find x and y coordinates for each block.
fn compute_positions(
    index: BlockId,
    graph: &[Block],
    blocks_dimensions: &[(f64, f64)],
    positions: &mut [(f64, f64)],
) {
    match graph[index] {
        Block::Sequence(ref s) => {
            // If it's a sequence, we move along y
            s.iter().fold(positions[index].1, |y, id| {
                // center on x
                let x_gap = (blocks_dimensions[index].0 - blocks_dimensions[*id].0) / 2.0;
                positions[*id] = (positions[index].0 + x_gap, y);
                compute_positions(*id, &graph, &blocks_dimensions, positions);
                y + blocks_dimensions[*id].1 + VERTICAL_GAP
            });
        }
        Block::Parallel(ref p) => {
            // If it's a parallel bloc, we move along x
            p.iter().fold(positions[index].0, |x, id| {
                // center on y
                let y_gap = (blocks_dimensions[index].1 - blocks_dimensions[*id].1) / 2.0;
                positions[*id] = (x, positions[index].1 + y_gap);
                compute_positions(*id, &graph, &blocks_dimensions, positions);
                x + blocks_dimensions[*id].0
            });
        }
        _ => (),
    }
}

/// Take a block ; fill its rectangles and edges and return a set of entry points for incoming edges
/// and a set of exit points for outgoing edges.
fn generate_visualisation(
    index: BlockId,
    graph: &[Block],
    positions: &[(f64, f64)],
    rectangles: &mut Vec<Rectangle>,
    edges: &mut Vec<(Point, Point)>,
) -> (Vec<Point>, Vec<Point>) {
    match graph[index] {
        Block::Sequence(ref s) => {
            let points: Vec<(Vec<Point>, Vec<Point>)> = s
                .iter()
                .map(|b| generate_visualisation(*b, graph, positions, rectangles, edges))
                .collect();
            edges.extend(
                points
                    .windows(2)
                    .flat_map(|w| iproduct!(w[0].1.iter(), w[1].0.iter()).map(|(a, b)| (*a, *b))),
            );
            (
                points.first().map(|p| &p.0).unwrap().clone(),
                points.last().map(|p| &p.1).unwrap().clone(),
            )
        }
        Block::Parallel(ref p) => p.iter().fold((Vec::new(), Vec::new()), |mut acc, b| {
            let (entry, exit) = generate_visualisation(*b, graph, positions, rectangles, edges);
            acc.0.extend(entry);
            acc.1.extend(exit);
            acc
        }),
        Block::Task(ref t) => {
            let duration = (t.end_time - t.start_time) as f64;
            rectangles.push(Rectangle::new(
                [0.0, 0.0, 0.0, 1.0],
                positions[index],
                (duration, 1.0),
                None,
            ));
            rectangles.push(Rectangle::new(
                COLORS[t.thread_id % COLORS.len()],
                positions[index],
                (duration, 1.0),
                Some((t.start_time, t.end_time)),
            ));

            (
                vec![(positions[index].0 + duration / 2.0, positions[index].1)],
                vec![(
                    positions[index].0 + duration / 2.0,
                    positions[index].1 + 1.0,
                )],
            )
        }
    }
}

/// convert all tasks information into animated rectangles and edges.
pub fn visualization(tasks: &[TaskLog]) -> (Vec<Rectangle>, Vec<(Point, Point)>) {
    // turn tasks into blocks (we build the fork join graph)
    let g = create_graph(tasks);

    // compute recursively the width and height of each block
    let mut blocks_dimensions = Vec::with_capacity(g.len());
    unsafe { blocks_dimensions.set_len(g.len()) }
    compute_blocks_dimensions(0, &g, &mut blocks_dimensions);

    // compute recursively the position of each block
    let mut positions = Vec::with_capacity(g.len());
    unsafe { positions.set_len(g.len()) }
    positions[0] = (0.0, 0.0);
    compute_positions(0, &g, &blocks_dimensions, &mut positions);

    // generate all rectangles and all edges
    let mut rectangles = Vec::with_capacity(2 * tasks.len());
    let mut edges = Vec::with_capacity(3 * tasks.len());
    generate_visualisation(0, &g, &positions, &mut rectangles, &mut edges);
    (rectangles, edges)
}
