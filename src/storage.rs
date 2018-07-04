//! provides a `Storage` structure with O(1) WORST CASE very fast insertions.
use std::cell::UnsafeCell;
use std::collections::LinkedList;
use RayonEvent;

const BLOCK_SIZE: usize = 10_000;

/// We store events in a list of blocks.
struct Block {
    data: Box<[RayonEvent]>,
    used: usize,
}

impl Block {
    /// Create a new block.
    fn new() -> Block {
        let mut v = Vec::with_capacity(BLOCK_SIZE);
        unsafe { v.set_len(BLOCK_SIZE) }
        Block {
            data: v.into_boxed_slice(),
            used: 0,
        }
    }

    /// Add given event to block.
    fn push(&mut self, event: RayonEvent) {
        assert!(self.used != BLOCK_SIZE);
        self.data[self.used] = event;
        self.used += 1;
    }

    /// Is there some space left.
    fn is_full(&self) -> bool {
        self.used == BLOCK_SIZE
    }

    /// Iterator on all logs.
    fn logs<'a>(&'a self) -> impl Iterator<Item = &'a RayonEvent> + 'a {
        (0..self.used).map(move |i| &self.data[i])
    }
}

/// Store logs here (in each thread).
pub(crate) struct Storage {
    data: UnsafeCell<LinkedList<Block>>,
}

unsafe impl Sync for Storage {}

impl Storage {
    /// Create a new storage space for logs.
    pub fn new() -> Self {
        let first_block = Block::new();
        let mut list = LinkedList::new();
        list.push_front(first_block);
        Storage {
            data: UnsafeCell::new(list),
        }
    }

    /// Destroy all logs.
    pub fn clear(&self) {
        let first_block = Block::new();
        let list = unsafe { self.data.get().as_mut() }.unwrap();
        list.clear();
        list.push_front(first_block);
    }

    /// Add given event to storage space.
    pub fn push(&self, event: RayonEvent) {
        let list = unsafe { self.data.get().as_mut() }.unwrap();
        let space_needed = list.front().unwrap().is_full();
        if space_needed {
            list.push_front(Block::new());
        }
        list.front_mut().unwrap().push(event)
    }

    /// Iterate on all logs inside us.
    pub(crate) fn logs<'a>(&self) -> impl Iterator<Item = &'a RayonEvent> + 'a {
        unsafe { self.data.get().as_ref() }
            .unwrap()
            .iter()
            .rev() // blocks are stored from newest to oldest
            .flat_map(|b| b.logs())
    }
}
