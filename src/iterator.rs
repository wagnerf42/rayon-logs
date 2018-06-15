//! Provides logging for parallel iterators.
use rayon::iter::plumbing::*;
use rayon::iter::*;
use time::precise_time_ns;
use {IteratorId, LoggedPool, RayonEvent, TaskId};

/// `Logged` is an iterator that logs all tasks created in a `LoggedPool`.
///
/// This struct is created by the [`logged()`] method on [`ParallelIterator`]
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
pub struct Logged<'a, I: ParallelIterator> {
    base: I,
    pool: &'a LoggedPool,
}

impl<'a, I: ParallelIterator> Logged<'a, I> {
    /// Create a new `Logged` iterator.
    pub(crate) fn new(base: I, pool: &'a LoggedPool) -> Logged<'a, I>
    where
        I: ParallelIterator,
    {
        Logged { base, pool }
    }
}

impl<'a, T, I> ParallelIterator for Logged<'a, I>
where
    I: ParallelIterator<Item = T>,
    T: Send,
{
    type Item = T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        let consumer1 = LoggedConsumer {
            base: consumer,
            pool: self.pool,
            part: None,
            iterator_id: self.pool.next_iterator_id(),
        };
        self.pool
            .log(RayonEvent::IteratorStart(consumer1.iterator_id));
        self.base.drive_unindexed(consumer1)
    }

    fn opt_len(&self) -> Option<usize> {
        self.base.opt_len()
    }
}

impl<'a, T, I> IndexedParallelIterator for Logged<'a, I>
where
    I: IndexedParallelIterator<Item = T>,
    T: Send,
{
    fn drive<C>(self, consumer: C) -> C::Result
    where
        C: Consumer<Self::Item>,
    {
        let part = Some((0, self.base.len()));
        let consumer1 = LoggedConsumer {
            base: consumer,
            pool: self.pool,
            part,
            iterator_id: self.pool.next_iterator_id(),
        };
        self.pool
            .log(RayonEvent::IteratorStart(consumer1.iterator_id));
        self.base.drive(consumer1)
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

struct LoggedConsumer<'a, C> {
    base: C,
    pool: &'a LoggedPool,
    part: Option<(usize, usize)>,
    iterator_id: IteratorId,
}

impl<'a, T, C> Consumer<T> for LoggedConsumer<'a, C>
where
    C: Consumer<T>,
    T: Send,
{
    type Folder = LoggedFolder<'a, C::Folder>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);
        let left_part = self.part.map(|(s, _)| (s, s + index));
        let right_part = self.part.map(|(s, e)| (s + index, e));
        (
            LoggedConsumer {
                base: left,
                pool: self.pool,
                part: left_part,
                iterator_id: self.iterator_id,
            },
            LoggedConsumer {
                base: right,
                pool: self.pool,
                part: right_part,
                iterator_id: self.iterator_id,
            },
            reducer,
        )
    }

    fn into_folder(self) -> LoggedFolder<'a, C::Folder> {
        let id = self.pool.next_task_id();

        self.pool.log(RayonEvent::TaskStart(
            id,
            precise_time_ns() - self.pool.start,
        ));
        self.pool
            .log(RayonEvent::IteratorTask(id, self.iterator_id, self.part));

        LoggedFolder {
            base: self.base.into_folder(),
            pool: self.pool,
            id,
        }
    }

    fn full(&self) -> bool {
        self.base.full()
    }
}

impl<'a, T, C> UnindexedConsumer<T> for LoggedConsumer<'a, C>
where
    C: UnindexedConsumer<T>,
    T: Send,
{
    fn split_off_left(&self) -> Self {
        LoggedConsumer {
            base: self.base.split_off_left(),
            pool: self.pool,
            part: None,
            iterator_id: self.iterator_id,
        }
    }

    fn to_reducer(&self) -> Self::Reducer {
        self.base.to_reducer()
    }
}

/// ////////////////////////////////////////////////////////////////////////
/// Folder implementation

struct LoggedFolder<'a, F> {
    base: F,
    pool: &'a LoggedPool,
    id: TaskId,
}

impl<'a, T, F> Folder<T> for LoggedFolder<'a, F>
where
    F: Folder<T>,
    T: Send,
{
    type Result = F::Result;

    fn consume(self, item: T) -> Self {
        LoggedFolder {
            base: self.base.consume(item),
            pool: self.pool,
            id: self.id,
        }
    }

    fn complete(self) -> F::Result {
        let result = self.base.complete();
        self.pool.log(RayonEvent::TaskEnd(
            self.id,
            precise_time_ns() - self.pool.start,
        ));
        result
    }

    fn full(&self) -> bool {
        self.base.full()
    }
}
