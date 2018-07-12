# rayon-logs

## Presentation

rayon-logs is a logging extension for the [rayon](https://github.com/rayon-rs/rayon) library.

The library is available on crates.io.

Logs are performance oriented and should hopefully enable users to enhance applications performances.

It can generated animate svg traces of the dag of executed tasks.

Github seems to filter out animated svgs so feel free to navigate my web page for some
[eye candy](http://www-id.imag.fr/Laboratoire/Membres/Wagner_Frederic/rayon-logs.html).

Here is an example of an svg animation of a merge sort's logs :
![merge sort animation](https://github.com/wagnerf42/rayon-logs/blob/master/merge_sort_sequential_merge.svg)

You can see all 4 threads executing the fork-join graph of the application with the idle times displayed on the right

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
