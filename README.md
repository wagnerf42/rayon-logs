# rayon-logs

## Presentation

rayon-logs is a logging extension for the [rayon](https://github.com/rayon-rs/rayon) library.

Logs are performance oriented and should hopefully enable users to enhance applications performances.

Here is an example of an svg animation of a merge sort's logs :
![merge sort animation](merge_sort_sequential_merge.svg)

You can see all 4 threads executing the fork-join graph of the application with the idle times displayed on the right
(click to animate).

analyzing this figure you can see:

- use of *join_context*
- the merge is not parallel which generates idle times
- no idle times related to *join_context*
- tasks decomposition generate no tangible overhead

Now, a second example comparing a run with a merge sort with sequential merge (top graph) and a
merge sort with parallel merge (bottom graph) (click to animate).
![merge sort parallel_animation](merge_sort_parallel_merge.svg)

Here I show more detail by refining sequential tasks further down into chains.
You can see that the parallel merge is improving on the idle times.
Moreover if you look closely you can see potential improvement points within rayon.
The bottom graph looks perfect but on a closer look you can see something not so nice:

- last level uses all 4 threads : seems perfect
- one level before you have green and red on the left and yellow and blue on the right
- again one level before you have yellow, red, blue, green

This means that when reaching next to last level the yellow thread will switch sides
with the green one, destroying the algorithm locality.
The impact on performances does not seem that big here since
all merge tasks are tagged as such and the displayed color would dim on large performance hits.


## Use

As of now, the documentation is very incomplete since the project is only starting.
Your best go is to take a look at the different examples provided.

It is intended to be imported INSTEAD of rayon and you should not call rayon functions directly nor import
its crate.

Logging of iterators is supported but only the *par_iter* method is currently available.

## Contact us

We would be **very interested** if your parallel applications have performance issues. Do not hesitate to contact us
for adding functions and or discussion.
