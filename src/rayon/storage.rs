//! provides a `Storage` structure with O(1) WORST CASE very fast insertions.
//! every thread has its own storage and will be the only one to write in it.
use super::list::AtomicLinkedList;

const BLOCK_SIZE: usize = 10_000;

/// We store elements in a list of blocks.
/// Each `Block` is a contiguous memory block.
struct Block<T> {
    data: Vec<T>,
}

impl<T> Default for Block<T> {
    fn default() -> Self {
        Block::new()
    }
}

impl<T> Block<T> {
    /// Create a new block.
    fn new() -> Self {
        Block {
            data: Vec::with_capacity(BLOCK_SIZE),
        }
    }

    /// Add given element to block.
    fn push(&mut self, element: T) {
        debug_assert!(self.data.len() != BLOCK_SIZE);
        self.data.push(element)
    }

    /// Is there some space left.
    fn is_full(&self) -> bool {
        self.data.len() == BLOCK_SIZE
    }

    /// Iterator on all elements.
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T> + 'a {
        self.data.iter()
    }
}

/// Fast structure (worst case O(1)) for pushing
/// logs in a thread.
pub struct Storage<T> {
    data: AtomicLinkedList<Block<T>>,
}

unsafe impl<T: Sync> Sync for Storage<T> {}

impl<T: 'static> Default for Storage<T> {
    fn default() -> Self {
        Storage::new()
    }
}

impl<T: 'static> Storage<T> {
    /// Create a new storage space.
    pub fn new() -> Self {
        let first_block = Block::new();
        let list = AtomicLinkedList::new();
        list.push_front(first_block);
        Storage { data: list }
    }

    /// Add given element to storage space.
    pub fn push(&self, element: T) {
        let space_needed = self.data.front().unwrap().is_full();
        if space_needed {
            self.data.push_front(Block::new());
        }
        self.data.front_mut().unwrap().push(element)
    }
    pub fn reset(&self) {
        self.data.reset();
        let first_block = Block::new();
        self.data.push_front(first_block);
    }
}

impl<T: 'static> Storage<T> {
    /// Iterate on all elements inside us.
    pub fn iter(&self) -> impl Iterator<Item = &'static T> + 'static {
        let blocks = self.data.iter().collect::<Vec<_>>();
        blocks.into_iter().rev().flat_map(|b| b.iter())
    }
}
