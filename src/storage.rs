//! provides a `Storage` structure with O(1) WORST CASE very fast insertions.
//! every thread has its own storage and will be the only one to write in it.
//! however after computations end, a master thread will extract all elements
//! from all storages. it thus requires an `UnsafeCell`.
use std::cell::UnsafeCell;
use std::collections::LinkedList;
use std::mem;
use std::ptr;

const BLOCK_SIZE: usize = 10_000;

/// We store elements in a list of blocks.
/// Each `Block` is a contiguous memory block.
struct Block<T> {
    data: [T; BLOCK_SIZE],
    used: usize,
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
            data: unsafe { mem::uninitialized() },
            used: 0,
        }
    }

    /// Add given element to block.
    fn push(&mut self, element: T) {
        debug_assert!(self.used != BLOCK_SIZE);
        unsafe { ptr::write(self.data.get_unchecked_mut(self.used), element) };
        self.used += 1;
    }

    /// Is there some space left.
    fn is_full(&self) -> bool {
        self.used == BLOCK_SIZE
    }

    /// Iterator on all elements.
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T> + 'a {
        (0..self.used).map(move |i| &self.data[i])
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

    /// Destroy all elements (does not free block memory but will drop all elements).
    pub fn clear(&self) {
        let list = unsafe { self.data.get().as_mut() }.unwrap();
        for block in list.iter_mut() {
            for index in 0..block.used {
                unsafe { ptr::drop_in_place(block.data.get_unchecked_mut(index)) }
            }
            block.used = 0;
        }
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
