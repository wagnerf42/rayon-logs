//! Store a trace as a fork join graph (in a vector).
use {svg::COLORS, Rectangle, TaskLog};
type BlockId = usize;
use itertools::repeat_call;
use std::collections::HashMap;
use std::iter::repeat;
use TimeStamp;

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

enum Block {
    Task(TaskLog),
    Sequence(Vec<BlockId>),
    Parallel(Vec<BlockId>),
}

/// Create a fork join graph (stored in a vec). This is used to convert the logs into
/// a graphical display of animated rectangles.
/// pre-condition : tasks need to be sorted BY TIME.
fn create_graph(tasks: &[TaskLog], threads_number: usize) -> Vec<Block> {
    // we are going to cheat by creating false tasks to display idle times on the right side of the
    // screen.
    // we have in parallel the graph on the left and the idle times on the right.
    let mut graph = vec![Block::Parallel(Vec::new())];
    let real_graph = graph.add_sequence(); // left
    let idle_times = graph.add_sequence(); // right
    graph.parallel(0).push(real_graph);
    graph.parallel(0).push(idle_times);
    let idle_blocks: Vec<BlockId> = repeat_call(|| {
        let idle_block = graph.add_parallel();
        graph.sequence(idle_times).push(idle_block); // one thread's idle times below the others
        idle_block
    }).take(threads_number)
        .collect();
    let mut last_activities: Vec<TimeStamp> = repeat(0).take(threads_number).collect();

    // ok let's start now
    let mut fathers: HashMap<BlockId, BlockId> = HashMap::new();
    let mut current_blocks = HashMap::new();
    current_blocks.insert(0, real_graph); // init task and all its descendants go in real graph

    for (task_id, task) in tasks.iter().enumerate() {
        eprintln!("id: {}", task_id);
        // start by idle times
        if task.start_time - last_activities[task.thread_id] > 0 {
            let false_idle_task = TaskLog {
                start_time: last_activities[task.thread_id],
                end_time: task.start_time,
                thread_id: task.thread_id,
                children: Vec::new(),
                work: None,
            };
            let block = graph.add_task(false_idle_task);
            graph.parallel(idle_blocks[task.thread_id]).push(block);
        }
        last_activities[task.thread_id] = task.end_time;

        // now continue with non-idle part
        let current_block = current_blocks[&task_id];

        // add task to its sequence
        let new_block = graph.add_task(task.clone());
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
        } else {
            let parallel_block = graph.add_parallel();
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

fn compute_blocks_widths(index: BlockId, graph: &[Block], blocks_widths: &mut [u64]) -> u64 {
    let sub = match graph[index] {
        Block::Sequence(ref s) => s
            .iter()
            .map(|id| compute_blocks_widths(*id, &graph, blocks_widths))
            .max()
            .unwrap(),
        Block::Parallel(ref p) => p
            .iter()
            .map(|id| compute_blocks_widths(*id, &graph, blocks_widths))
            .sum(),
        Block::Task(ref t) => t.end_time - t.start_time,
    };
    blocks_widths[index] = sub;
    blocks_widths[index]
}

fn compute_rectangles(
    index: BlockId,
    graph: &[Block],
    blocks_widths: &[u64],
    x_offset: f64,
    y_offset: f64,
    rectangles: &mut Vec<Rectangle>,
) -> (f64, f64) {
    match graph[index] {
        Block::Sequence(ref s) => {
            // If it's a sequence, we don't care about the x_offset
            (
                x_offset,
                s.iter().fold(y_offset, |y, id| {
                    y + compute_rectangles(*id, &graph, &blocks_widths, x_offset, y, rectangles).1
                }),
            )
        }
        Block::Parallel(ref p) => {
            // If it's a parallel bloc, we don't care about the y_offset
            (
                p.iter().fold(x_offset, |x, block_id| {
                    x + compute_rectangles(
                        *block_id,
                        &graph,
                        &blocks_widths,
                        x,
                        y_offset,
                        rectangles,
                    ).0
                }),
                y_offset,
            )
        }
        Block::Task(ref t) => {
            let width = (t.end_time - t.start_time) as f64;
            let rec = Rectangle::new(
                COLORS[t.thread_id % 8],
                (blocks_widths[index] as f64 - width) / 2.0 + x_offset,
                y_offset,
                width,
                1.0,
                t.start_time,
                t.end_time,
            );
            rectangles.push(rec);
            (x_offset + width, y_offset + 1.0 + 1.5)
        }
    }
}

/// convert all tasks information into animated rectangles.
pub fn visualization_rectangles(tasks: &[TaskLog], threads_number: usize) -> Vec<Rectangle> {
    // turn tasks into blocks (we build the fork join graph)
    let g = create_graph(tasks, threads_number);
    let mut blocks_widths = Vec::with_capacity(g.len());
    unsafe { blocks_widths.set_len(g.len()) }

    // compute recursively the width of each block
    compute_blocks_widths(0, &g, &mut blocks_widths);

    // position x and y coordinates of each block and generate all animated rectangles
    let mut rectangles = Vec::with_capacity(tasks.len() * 2);
    compute_rectangles(0, &g, &blocks_widths, 0.0, 0.0, &mut rectangles);
    rectangles
}
