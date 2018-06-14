use rayon::iter::plumbing::*;
use rayon::iter::*;

/// `Logged` is an iterator that logs all tasks created in a `LoggedPool`.
///
/// This struct is created by the [`logged()`] method on [`ParallelIterator`]
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
#[derive(Debug, Clone)]
pub struct Logged<I: ParallelIterator> {
    base: I,
}

/// Create a new `Logged` iterator.
///
/// NB: a free fn because it is NOT part of the end-user API.
pub fn new<I>(base: I) -> Logged<I>
where
    I: ParallelIterator,
{
    Logged { base: base }
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
        let consumer1 = LoggedConsumer::new(consumer);
        self.base.drive_unindexed(consumer1)
    }

    fn opt_len(&self) -> Option<usize> {
        self.base.opt_len()
    }
}

impl<'a, T, I> IndexedParallelIterator for Logged<I>
where
    I: IndexedParallelIterator<Item = &'a T>,
    T: 'a + Send + Sync,
{
    fn drive<C>(self, consumer: C) -> C::Result
    where
        C: Consumer<Self::Item>,
    {
        let consumer1 = LoggedConsumer::new(consumer);
        self.base.drive(consumer1)
    }

    fn len(&self) -> usize {
        self.base.len()
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        return self.base.with_producer(Callback { callback: callback });

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
                let producer = LoggedProducer { base: base };
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
}

impl<C> LoggedConsumer<C> {
    fn new(base: C) -> Self {
        LoggedConsumer { base: base }
    }
}

impl<'a, T, C> Consumer<T> for LoggedConsumer<C>
where
    C: Consumer<T>,
    T: Send,
{
    type Folder = C::Folder;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);
        println!("splitting at {}", index);
        (
            LoggedConsumer::new(left),
            LoggedConsumer::new(right),
            reducer,
        )
    }

    fn into_folder(self) -> Self::Folder {
        println!("into folder");
        self.base.into_folder()
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
        LoggedConsumer::new(self.base.split_off_left())
    }

    fn to_reducer(&self) -> Self::Reducer {
        self.base.to_reducer()
    }
}
