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
    pub fn new(base: I) -> Logged<I>
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
        let consumer_id = next_task_id();
        let iterator_id = next_iterator_id();
        let consumer1 = LoggedConsumer {
            base: consumer,
            part: self.base.opt_len().map(|l| (0, l)),
            iterator_id,
            consumer_id,
            continuing_task_id,
        };
        //log(RayonEvent::IteratorStart(consumer1.iterator_id));
        log(RayonEvent::Child(consumer_id));
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
        let consumer_id = next_task_id();
        let iterator_id = next_iterator_id();
        let consumer1 = LoggedConsumer {
            base: consumer,
            part,
            iterator_id,
            consumer_id,
            continuing_task_id,
        };
        //log(RayonEvent::IteratorStart(consumer1.iterator_id));
        log(RayonEvent::Child(consumer_id));
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
    consumer_id: TaskId,
    continuing_task_id: TaskId,
}

impl<T, C> Consumer<T> for LoggedConsumer<C>
where
    C: Consumer<T>,
    T: Send,
{
    type Folder = LoggedFolder<C::Folder>;
    type Reducer = LoggedReducer<C::Reducer>;
    type Result = C::Result;

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let consumer_id_1 = next_task_id();
        let consumer_id_2 = next_task_id();
        let continuing_reducer_id = next_task_id();
        log(RayonEvent::TaskStart(self.consumer_id, precise_time_ns()));
        log(RayonEvent::Child(consumer_id_1));
        log(RayonEvent::Child(consumer_id_2));
        let (left, right, reducer) = self.base.split_at(index);
        let left_part = self.part.map(|(s, _)| (s, s + index));
        let right_part = self.part.map(|(s, e)| (s + index, e));
        let r = (
            LoggedConsumer {
                base: left,
                part: left_part,
                iterator_id: self.iterator_id,
                consumer_id: consumer_id_1,
                continuing_task_id: continuing_reducer_id,
            },
            LoggedConsumer {
                base: right,
                part: right_part,
                iterator_id: self.iterator_id,
                consumer_id: consumer_id_2,
                continuing_task_id: continuing_reducer_id,
            },
            LoggedReducer {
                rayon_reducer: reducer,
                id: continuing_reducer_id,
                continuing_task_id: self.continuing_task_id,
            },
        );
        log(RayonEvent::TaskEnd(precise_time_ns()));
        r
    }

    fn into_folder(self) -> LoggedFolder<C::Folder> {
        log(RayonEvent::TaskStart(self.consumer_id, precise_time_ns()));
        //log(RayonEvent::IteratorTask(
        //    self.consumer_id,
        //    self.iterator_id,
        //    self.part,
        //    continuing_id,
        //));

        LoggedFolder {
            base: self.base.into_folder(),
            continuing_task_id: self.continuing_task_id,
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
        let split_task_id = next_task_id();
        let continuing_task_id = next_task_id();
        log(RayonEvent::TaskStart(split_task_id, precise_time_ns()));
        let consumer_id = next_task_id();
        //log(RayonEvent::Join(id, second_id, self.continuing_task_id));
        let r = LoggedConsumer {
            base: self.base.split_off_left(),
            part: None,
            iterator_id: self.iterator_id,
            consumer_id,
            continuing_task_id,
        };
        log(RayonEvent::TaskEnd(precise_time_ns()));
        r
    }
    fn to_reducer(&self) -> LoggedReducer<C::Reducer> {
        let reducer_id = next_task_id();
        LoggedReducer {
            rayon_reducer: self.base.to_reducer(),
            id: reducer_id,
            continuing_task_id: self.continuing_task_id,
        }
    }
}

/// ////////////////////////////////////////////////////////////////////////
/// Folder implementation

struct LoggedFolder<F> {
    base: F,
    continuing_task_id: TaskId,
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
            continuing_task_id: self.continuing_task_id,
        }
    }

    fn complete(self) -> F::Result {
        let result = self.base.complete();
        log(RayonEvent::Child(self.continuing_task_id));
        log(RayonEvent::TaskEnd(precise_time_ns()));
        result
    }

    fn full(&self) -> bool {
        self.base.full()
    }
}

/// Logged Reducer struct implementation.

struct LoggedReducer<R> {
    rayon_reducer: R,
    id: TaskId,
    continuing_task_id: TaskId,
}

impl<T, R> Reducer<T> for LoggedReducer<R>
where
    R: Reducer<T>,
    T: Send,
{
    fn reduce(self, left: T, right: T) -> T {
        log(RayonEvent::TaskStart(self.id, precise_time_ns()));
        let r = self.rayon_reducer.reduce(left, right);
        log(RayonEvent::Child(self.continuing_task_id));
        log(RayonEvent::TaskEnd(precise_time_ns()));
        r
    }
}
