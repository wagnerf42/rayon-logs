//! Store a trace as a fork join graph (in a vector).
use super::svg::{Point, Rectangle, Scene, COLORS};
use crate::{TaskId, TimeStamp};
type BlockId = usize;
use crate::log::{RunLog, TaskLog};
use itertools::{iproduct, Itertools};
use std::collections::HashMap;
use std::iter::{once, repeat};

const VERTICAL_GAP: f64 = 0.2;

trait BlockVector {
    fn push_block(&mut self, block: Block) -> BlockId;
    fn sequence(&mut self, id: BlockId) -> &mut Vec<BlockId>;
    fn read_sequence(&self, id: BlockId) -> &Vec<BlockId>;
    fn task(&self, id: BlockId) -> (TaskId, &TaskLog);
    fn parallel(&mut self, id: BlockId) -> &mut Vec<BlockId>;
    fn add_task(&mut self, task_id: TaskId, task: TaskLog) -> BlockId;
    fn add_sequence(&mut self) -> BlockId;
    fn add_parallel(&mut self) -> BlockId;
}

impl BlockVector for Vec<Block> {
    fn add_task(&mut self, task_id: TaskId, task: TaskLog) -> BlockId {
        self.push_block(Block::Task(task_id, task))
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
    fn read_sequence(&self, id: BlockId) -> &Vec<BlockId> {
        match self[id] {
            Block::Sequence(ref s) => s,
            _ => panic!("should be a sequence"),
        }
    }
    fn task(&self, id: BlockId) -> (TaskId, &TaskLog) {
        match self[id] {
            Block::Task(task_id, ref task_log) => (task_id, task_log),
            _ => panic!("should be a task"),
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
pub(crate) enum Block {
    Task(TaskId, TaskLog),
    Sequence(Vec<BlockId>),
    Parallel(Vec<BlockId>),
}

/// iterate on all ancestors blocks (including initial block)
fn ancestors_blocks<'a>(
    blocks_fathers: &'a HashMap<BlockId, BlockId>,
    block: BlockId,
) -> impl Iterator<Item = BlockId> + 'a {
    (0..).scan(block, move |b, _| {
        blocks_fathers
            .get(b)
            .map(|f| {
                let current_b = *b;
                *b = *f;
                current_b
            })
            .or_else(|| Some(*b)) // it repeats on last but I guess it's ok
    })
}

/// find first block common ancestor of two blocks
fn common_ancestor_block(
    blocks_fathers: &HashMap<BlockId, BlockId>,
    blocks: &[BlockId],
) -> Option<BlockId> {
    blocks
        .iter()
        .map(|b| ancestors_blocks(blocks_fathers, *b))
        .kmerge_by(|b1, b2| b1 > b2) // blocks order is topological order
        .tuples()
        .find(|(b1, b2)| b1 == b2)
        .map(|(b1, _)| b1)
}

/// Create a fork join graph (stored in a vec). This is used to convert the logs into
/// a graphical display of animated rectangles.
/// Returns the fork join graph and a vector containing all root blocks, sorted by starting times.
pub(crate) fn create_graph(tasks: &[TaskLog]) -> (Vec<Block>, Vec<BlockId>) {
    // graph is composed of sequential or parallel blocks
    let mut graph = Vec::with_capacity(tasks.len());
    let mut root_nodes = Vec::new();

    // we need a quick way to find all fathers for a given node
    let mut fathers: HashMap<TaskId, Vec<TaskId>> = HashMap::new();
    for (task_id, task) in tasks.iter().enumerate() {
        for child in &task.children {
            fathers.entry(*child).or_insert_with(Vec::new).push(task_id);
        }
    }

    // now, we are going to compute in which block is every node
    let mut blocks: HashMap<TaskId, BlockId> = HashMap::new();

    // store all parallel blocks (one for each multi-children node)
    let mut parallel_blocks: HashMap<TaskId, BlockId> = HashMap::new();

    // also store what is the father block of each block
    let mut blocks_fathers: HashMap<BlockId, BlockId> = HashMap::new();

    // we sort by starting time to be sure fathers are processed before children
    let mut sorted_tasks: Vec<TaskId> = (0..tasks.len()).collect();
    sorted_tasks.sort_by(|t1, t2| {
        tasks[*t1]
            .start_time
            .partial_cmp(&tasks[*t2].start_time)
            .unwrap()
    });

    for task_id in &sorted_tasks {
        let task = &tasks[*task_id];
        let sequence_id = if !fathers.contains_key(task_id) {
            // we are one of the roots
            let block_id = graph.len();
            graph.push(Block::Sequence(Vec::new()));
            root_nodes.push((task_id, block_id));
            block_id
        } else if fathers[task_id].len() == 1 {
            // check if we have brothers
            let father = fathers[task_id][0];
            if tasks[father].children.len() == 1 {
                // one father, no brothers, we go directly after him
                // in his block
                blocks[&father]
            } else {
                // several brothers, we need to create a new sequence
                let sequential_block = graph.add_sequence();
                blocks_fathers.insert(sequential_block, blocks[&father]); // save where to go back
                let parallel_block = parallel_blocks[&father];
                graph.parallel(parallel_block).push(sequential_block);
                sequential_block
            }
        } else {
            // several fathers
            // we need to find the first (while going up) common ancestor block
            let mut direct_fathers_blocks = fathers[task_id].iter().map(|f| blocks[f]);
            let starting_block = direct_fathers_blocks.next().unwrap();
            direct_fathers_blocks.fold(starting_block, |b1, b2| {
                common_ancestor_block(&blocks_fathers, &[b1, b2]).expect("no common ancestor")
            })
        };
        let new_block = graph.add_task(*task_id, (*task).clone());
        graph.sequence(sequence_id).push(new_block);
        blocks.insert(*task_id, sequence_id);

        // now create a parallel block after us when we have multiple children
        if task.children.len() > 1 {
            // several children, we create a parallel block
            let parallel_block = graph.add_parallel();
            // add it to our block
            graph.sequence(blocks[task_id]).push(parallel_block);
            parallel_blocks.insert(*task_id, parallel_block);
        }
    }
    root_nodes.sort_by_key(|(t, _)| tasks[**t].start_time);
    (graph, root_nodes.into_iter().map(|(_, b)| b).collect())
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
        Block::Task(_, ref t) => ((t.end_time - t.start_time) as f64, 1.0),
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
    graph: &Vec<Block>,
    positions: &[(f64, f64)],
    scene: &mut Scene,
    tasks_information: &mut HashMap<TaskId, HashMap<String, (String, f64)>>,
    tags: &[String],
    blocks_dimensions: &[(f64, f64)],
) -> (Vec<Point>, Vec<Point>) {
    match graph[index] {
        Block::Sequence(ref s) => {
            let points: Vec<(Vec<Point>, Vec<Point>)> = s
                .iter()
                .map(|b| {
                    generate_visualisation(
                        *b,
                        graph,
                        positions,
                        scene,
                        tasks_information,
                        tags,
                        blocks_dimensions,
                    )
                })
                .collect();
            scene.segments.extend(
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
            let (entry, exit) = generate_visualisation(
                *b,
                graph,
                positions,
                scene,
                tasks_information,
                tags,
                blocks_dimensions,
            );
            acc.0.extend(entry);
            acc.1.extend(exit);
            acc
        }),
        Block::Task(task_id, ref t) => {
            let duration = (t.end_time - t.start_time) as f64;
            let information = tasks_information.remove(&task_id).unwrap();
            scene.rectangles.push(Rectangle::new(
                COLORS[t.thread_id % COLORS.len()],
                positions[index],
                (duration, 1.0),
                (t.start_time, t.end_time),
                information,
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

/// Take all taskslogs and compute idle periods animations for each thread.
/// add all rectangles to given vector.
/// given height (height of animated running tasks) enables us to center the display vertically.
/// y is vertical start for this log.
fn compute_idle_times(
    tasks: &[TaskLog],
    starting_position: &(f64, f64),
    threads_number: usize,
    scene: &mut Scene,
) {
    // do one pass to figure out the last recorded time.
    // we need it to figure out who is idle at the end.
    let last_time = tasks.iter().map(|t| t.end_time).max().unwrap_or(0);
    let first_time = tasks.iter().map(|t| t.start_time).min().unwrap_or(0);

    // sort everyone by time (yes i know, again).
    // we add fake tasks at the end for last idle periods.
    let mut sorted_tasks: Vec<(usize, TimeStamp, TimeStamp)> = tasks
        .iter()
        .map(|t| (t.thread_id, t.start_time, t.end_time))
        .chain((0..threads_number).map(|i| (i, last_time, last_time + 1)))
        .collect();

    sorted_tasks.sort_by(|t1, t2| t1.1.partial_cmp(&t2.1).unwrap());

    let mut previous_activities: Vec<TimeStamp> = repeat(first_time).take(threads_number).collect();
    let mut current_x_positions: Vec<f64> =
        repeat(starting_position.0).take(threads_number).collect();

    // replay execution, figuring out idle times
    for (thread_id, start, end) in sorted_tasks {
        let previous_end = previous_activities[thread_id];
        if start > previous_end {
            let inactivity = (start - previous_end) as f64;
            scene.rectangles.push(Rectangle::new(
                COLORS[thread_id % COLORS.len()],
                (
                    current_x_positions[thread_id],
                    starting_position.1 + thread_id as f64 * (1.0 + VERTICAL_GAP),
                ),
                (inactivity, 1.0),
                (previous_end, start),
                once(("_NO_TAGS_".to_string(), ("idle".to_string(), 1.0))).collect(),
            ));
            current_x_positions[thread_id] += inactivity;
        }
        previous_activities[thread_id] = end;
    }
}

/// Computes a graphical view of a log. This is intended for the development of logs viewers.
pub(crate) fn visualisation(log: &RunLog) -> Scene {
    let mut scene = Scene::new(log);

    let tasks = &log.tasks_logs;
    let (g, root_blocks) = create_graph(tasks);

    // compute recursively the width and height of each block
    let mut blocks_dimensions = Vec::with_capacity(g.len());
    unsafe { blocks_dimensions.set_len(g.len()) }
    for root in &root_blocks {
        compute_blocks_dimensions(*root, &g, &mut blocks_dimensions);
    }

    // compute the width to center everyone
    let width = root_blocks
        .iter()
        .map(|b| blocks_dimensions[*b].0)
        .max_by(|a, b| a.partial_cmp(&b).unwrap())
        .unwrap_or(0.0);

    // compute recursively the position of each block
    let mut positions = Vec::with_capacity(g.len());
    unsafe { positions.set_len(g.len()) }
    let height = root_blocks.iter().fold(0.0, |previous_h, root| {
        positions[*root] = ((width - blocks_dimensions[*root].0) / 2.0, previous_h);
        compute_positions(*root, &g, &blocks_dimensions, &mut positions);
        previous_h + blocks_dimensions[*root].1 + 1.0
    });

    let mut tasks_information = log.compute_tasks_information();
    for root in &root_blocks {
        generate_visualisation(
            *root,
            &g,
            &positions,
            &mut scene,
            &mut tasks_information,
            &log.tags,
            &blocks_dimensions,
        );
    }

    let starting_position = (width as f64 * 0.1, height + 1.0);

    compute_idle_times(tasks, &starting_position, log.threads_number, &mut scene);

    scene
}
