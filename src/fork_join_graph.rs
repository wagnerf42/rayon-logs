//! Store a trace as a fork join graph (in a vector).
use {svg::COLORS, Rectangle, TaskId, TaskLog};
type BlockId = usize;
use std::cmp::max;
use std::collections::HashMap;

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
/// pre-condition: tasks form a topological order.
fn create_graph(tasks: &[TaskLog]) -> Vec<Block> {
    let mut graph = vec![Block::Sequence(Vec::new())];

    // ok let's start now
    let mut fathers: HashMap<BlockId, BlockId> = HashMap::new();
    let mut current_blocks: HashMap<TaskId, BlockId> = HashMap::new();
    current_blocks.insert(0, 0); // init task and all its descendants go in initial sequence

    for (task_id, task) in tasks.iter().enumerate() {
        let current_block = current_blocks[&task_id];

        // add task to its sequence
        let new_block = graph.add_task((*task).clone());
        graph.sequence(current_blocks[&task_id]).push(new_block);

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
            graph
                .sequence(current_blocks[&task_id])
                .push(parallel_block);
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
        Block::Sequence(ref s) => s.iter().fold((0.0, 0.0), |dimensions, id| {
            let (width, height) = compute_blocks_dimensions(*id, &graph, blocks_dimensions);
            (
                if width > dimensions.0 {
                    width
                } else {
                    dimensions.0
                },
                height + dimensions.1,
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

/// Fill rectangles vector by propagating x and y positions.
fn compute_rectangles(
    index: BlockId,
    graph: &[Block],
    blocks_dimensions: &[(f64, f64)],
    x_offset: f64,
    y_offset: f64,
    rectangles: &mut Vec<Rectangle>,
) {
    match graph[index] {
        Block::Sequence(ref s) => {
            // If it's a sequence, we don't care about the x_offset
            s.iter().fold(y_offset, |y, id| {
                compute_rectangles(*id, &graph, &blocks_dimensions, x_offset, y, rectangles);
                y + blocks_dimensions[*id].1
            });
        }
        Block::Parallel(ref p) => {
            // If it's a parallel bloc, we don't care about the y_offset
            p.iter().fold(x_offset, |x, id| {
                compute_rectangles(*id, &graph, &blocks_dimensions, x, y_offset, rectangles);
                x + blocks_dimensions[*id].0
            });
        }
        Block::Task(ref t) => {
            let width = t.end_time - t.start_time;
            let rec = Rectangle::new(
                COLORS[t.thread_id % 8],
                x_offset,
                y_offset,
                width as f64,
                2.0,
                t.start_time,
                t.end_time,
            );
            rectangles.push(rec);
        }
    }
}

/// convert all tasks information into animated rectangles.
pub fn visualization_rectangles(tasks: &[TaskLog]) -> Vec<Rectangle> {
    // turn tasks into blocks (we build the fork join graph)
    let g = create_graph(tasks);
    let mut blocks_dimensions = Vec::with_capacity(g.len());
    unsafe { blocks_dimensions.set_len(g.len()) }

    // compute recursively the width of each block
    compute_blocks_dimensions(0, &g, &mut blocks_dimensions);

    // position x and y coordinates of each block and generate all animated rectangles
    let mut rectangles = Vec::with_capacity(tasks.len() * 2);
    compute_rectangles(0, &g, &blocks_dimensions, 0.0, 0.0, &mut rectangles);
    rectangles
}
