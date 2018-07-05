# rayon-logs

## Presentation

rayon-logs is a logging extension for the [rayon](https://github.com/rayon-rs/rayon) library.

logs are performance oriented and should hopefully enable users to enhance applications performances.

here is an example of an svg animation of a merge sort's logs :
![merge sort animation](merge_sort_sequential_merge.svg)

you can see all 4 threads executing the fork-join graph of the application with the idle times displayed on the right
(refresh the page to restart the animation).

analyzing this figure you can see:

- use of *join_context*
- the merge is not parallel which generates idle times
- no idle times related to *join_context*
- tasks decomposition generate no tangible overhead

## Use

As of now, the documentation is very incomplete since the project is only starting.
Your best go is to take a look at the different examples provided.

It is intended to be imported INSTEAD of rayon and you should not call rayon functions directly nor import
its crate.

Logging of iterators is supported but only the *par_iter* method is currently available.

## Contact us

We would be **very interested** if your parallel applications have performance issues. Do not hesitate to contact us
for adding functions and or discussion.
