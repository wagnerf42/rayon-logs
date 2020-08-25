//! This module defines an small atomic linked list.
//! It is safe as long as pushes are serialized which
//! is the case for our use since only one thread pushes.
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};

struct Node<T> {
    element: T,
    next: AtomicPtr<Node<T>>,
}

pub(crate) struct AtomicLinkedList<T> {
    head: AtomicPtr<Node<T>>,
}

impl<T: 'static> AtomicLinkedList<T> {
    pub(crate) fn new() -> Self {
        AtomicLinkedList {
            head: AtomicPtr::new(null_mut()),
        }
    }
    pub(crate) fn reset(&self) {
        let mut node_pointer = self.head.swap(null_mut(), Ordering::SeqCst);
        while let Some(node) = unsafe { node_pointer.as_ref() } {
            let old_node_pointer = node_pointer;
            node_pointer = node.next.load(Ordering::SeqCst);
            unsafe { old_node_pointer.drop_in_place() }
        }
    }
    pub(crate) fn push_front(&self, elt: T) {
        let new_node = Box::new(Node {
            element: elt,
            next: AtomicPtr::new(self.head.load(Ordering::SeqCst)),
        });
        self.head.store(Box::into_raw(new_node), Ordering::SeqCst)
    }
    pub(crate) fn front(&self) -> Option<&T> {
        unsafe { self.head.load(Ordering::Relaxed).as_ref() }.map(|n| &n.element)
    }
    pub(crate) fn front_mut(&self) -> Option<&mut T> {
        unsafe { self.head.load(Ordering::Relaxed).as_mut() }.map(|n| &mut n.element)
    }
    pub(crate) fn iter(&self) -> AtomicLinkedListIterator<T> {
        AtomicLinkedListIterator {
            current_node: self.head.load(Ordering::Relaxed),
        }
    }
}

pub(crate) struct AtomicLinkedListIterator<T> {
    current_node: *mut Node<T>,
}

impl<T: 'static> Iterator for AtomicLinkedListIterator<T> {
    type Item = &'static T;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = unsafe { self.current_node.as_ref() } {
            self.current_node = node.next.load(Ordering::Relaxed);
            Some(&node.element)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn push_front_test() {
        let list = AtomicLinkedList::new();
        list.push_front(1);
        list.push_front(2);
        list.push_front(3);
        assert!(list.iter().eq(vec![3, 2, 1].iter()))
    }
}
