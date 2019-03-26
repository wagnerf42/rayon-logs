//! provides a `Storage` structure with O(1) WORST CASE very fast insertions.
//! every thread has its own storage and will be the only one to write in it.
//! however after computations end, a master thread will extract all elements
//! from all storages. it thus requires an `UnsafeCell`.
use std::cell::UnsafeCell;
use std::collections::LinkedList;

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
pub(crate) struct Storage<T> {
    data: UnsafeCell<LinkedList<Block<T>>>,
}

unsafe impl<T: Sync> Sync for Storage<T> {}

impl<T> Default for Storage<T> {
    fn default() -> Self {
        Storage::new()
    }
}

impl<T> Storage<T> {
    /// Create a new storage space.
    pub fn new() -> Self {
        let first_block = Block::new();
        let mut list = LinkedList::new();
        list.push_front(first_block);
        Storage {
            data: UnsafeCell::new(list),
        }
    }

    /// Destroy all elements (frees all block memory).
    pub fn clear(&self) {
        let list = unsafe { self.data.get().as_mut() }.unwrap();
        list.clear();
        let first_block = Block::new();
        list.push_front(first_block);
    }

    /// Add given element to storage space.
    pub fn push(&self, element: T) {
        let list = unsafe { self.data.get().as_mut() }.unwrap();
        let space_needed = list.front().unwrap().is_full();
        if space_needed {
            list.push_front(Block::new());
        }
        list.front_mut().unwrap().push(element)
    }
}

impl<'a, T: 'a> Storage<T> {
    /// Iterate on all elements inside us.
    pub fn iter(&self) -> impl Iterator<Item = &'a T> + 'a {
        unsafe { self.data.get().as_ref() }
            .unwrap()
            .iter()
            .rev() // blocks are stored from newest to oldest
            .flat_map(|b| b.iter())
    }
}
