//! Provides logging for parallel iterators.
use pool::{log, next_iterator_id, next_task_id};
use rayon::iter::plumbing::*;
use rayon::iter::*;
use time::precise_time_ns;
use {IteratorId, RayonEvent, TaskId};

/// `Logged` is an iterator that logs all tasks created in a `LoggedPool`.
///
/// This struct is created by the [`logged()`] method on [`ParallelIterator`]
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
pub struct Logged<I: ParallelIterator> {
    base: I,
}

impl<I: ParallelIterator> Logged<I> {
    /// Create a new `Logged` iterator.
    pub(crate) fn new(base: I) -> Logged<I>
    where
        I: ParallelIterator,
    {
        Logged { base }
    }
}

impl<T, I> ParallelIterator for Logged<I>
where
    I: ParallelIterator<Item = T>,
    T: Send,
{
    type Item = T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        let continuing_task_id = next_task_id();
        let iterator_id = next_iterator_id();
        let consumer1 = LoggedConsumer {
            base: consumer,
            part: self.base.opt_len().map(|l| (0, l)),
            iterator_id,
            continuing_task_id,
        };
        log(RayonEvent::IteratorStart(consumer1.iterator_id));
        log(RayonEvent::TaskEnd(precise_time_ns()));
        let r = self.base.drive_unindexed(consumer1);
        log(RayonEvent::TaskStart(continuing_task_id, precise_time_ns()));
        r
    }

    fn opt_len(&self) -> Option<usize> {
        self.base.opt_len()
    }
}

impl<T, I> IndexedParallelIterator for Logged<I>
where
    I: IndexedParallelIterator<Item = T>,
    T: Send,
{
    fn drive<C>(self, consumer: C) -> C::Result
    where
        C: Consumer<Self::Item>,
    {
        let part = Some((0, self.base.len()));
        let continuing_task_id = next_task_id();
        let iterator_id = next_iterator_id();
        let consumer1 = LoggedConsumer {
            base: consumer,
            part,
            iterator_id,
            continuing_task_id,
        };
        log(RayonEvent::IteratorStart(consumer1.iterator_id));
        log(RayonEvent::TaskEnd(precise_time_ns()));
        let r = self.base.drive(consumer1);
        log(RayonEvent::TaskStart(continuing_task_id, precise_time_ns()));
        r
    }

    fn len(&self) -> usize {
        self.base.len()
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        return self.base.with_producer(Callback { callback });

        struct Callback<CB> {
            callback: CB,
        }

        impl<T, CB> ProducerCallback<T> for Callback<CB>
        where
            CB: ProducerCallback<T>,
            T: Send,
        {
            type Output = CB::Output;

            fn callback<P>(self, base: P) -> CB::Output
            where
                P: Producer<Item = T>,
            {
                let producer = LoggedProducer { base };
                self.callback.callback(producer)
            }
        }
    }
}

/// ////////////////////////////////////////////////////////////////////////

struct LoggedProducer<P> {
    base: P,
}

impl<T, P> Producer for LoggedProducer<P>
where
    P: Producer<Item = T>,
    T: Send,
{
    type Item = T;
    type IntoIter = P::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter()
    }

    fn min_len(&self) -> usize {
        self.base.min_len()
    }

    fn max_len(&self) -> usize {
        self.base.max_len()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.split_at(index);
        (
            LoggedProducer { base: left },
            LoggedProducer { base: right },
        )
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        self.base.fold_with(folder)
    }
}

/// ////////////////////////////////////////////////////////////////////////
/// Consumer implementation

struct LoggedConsumer<C> {
    base: C,
    part: Option<(usize, usize)>,
    iterator_id: IteratorId,
    /// which task comes after the iterator finishes (to mark dependencies)
    continuing_task_id: TaskId,
}

impl<T, C> Consumer<T> for LoggedConsumer<C>
where
    C: Consumer<T>,
    T: Send,
{
    type Folder = LoggedFolder<C::Folder>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);
        let left_part = self.part.map(|(s, _)| (s, s + index));
        let right_part = self.part.map(|(s, e)| (s + index, e));
        (
            LoggedConsumer {
                base: left,
                part: left_part,
                iterator_id: self.iterator_id,
                continuing_task_id: self.continuing_task_id,
            },
            LoggedConsumer {
                base: right,
                part: right_part,
                iterator_id: self.iterator_id,
                continuing_task_id: self.continuing_task_id,
            },
            reducer,
        )
    }

    fn into_folder(self) -> LoggedFolder<C::Folder> {
        let id = next_task_id();

        log(RayonEvent::TaskStart(id, precise_time_ns()));
        log(RayonEvent::IteratorTask(
            id,
            self.iterator_id,
            self.part,
            self.continuing_task_id,
        ));

        LoggedFolder {
            base: self.base.into_folder(),
            id,
        }
    }

    fn full(&self) -> bool {
        self.base.full()
    }
}

impl<T, C> UnindexedConsumer<T> for LoggedConsumer<C>
where
    C: UnindexedConsumer<T>,
    T: Send,
{
    fn split_off_left(&self) -> Self {
        LoggedConsumer {
            base: self.base.split_off_left(),
            part: None,
            iterator_id: self.iterator_id,
            continuing_task_id: self.continuing_task_id,
        }
    }

    fn to_reducer(&self) -> Self::Reducer {
        self.base.to_reducer()
    }
}

/// ////////////////////////////////////////////////////////////////////////
/// Folder implementation

struct LoggedFolder<F> {
    base: F,
    id: TaskId,
}

impl<T, F> Folder<T> for LoggedFolder<F>
where
    F: Folder<T>,
    T: Send,
{
    type Result = F::Result;

    fn consume(self, item: T) -> Self {
        LoggedFolder {
            base: self.base.consume(item),
            id: self.id,
        }
    }

    fn complete(self) -> F::Result {
        let result = self.base.complete();
        log(RayonEvent::TaskEnd(precise_time_ns()));
        result
    }

    fn full(&self) -> bool {
        self.base.full()
    }
}
