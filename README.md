# rayon-logs

## Presentation

rayon-logs is a logging extension for the [rayon](https://github.com/rayon-rs/rayon) library.

The library is available on crates.io.

Logs are performance oriented and should hopefully enable users to enhance applications performances.

It can generated animate svg traces of the dag of executed tasks.

Github seems to filter out animated svgs so feel free to navigate my web page for some
[eye](http://www-id.imag.fr/Laboratoire/Membres/Wagner_Frederic/rayon-adaptive.html).
[candy](http://www-id.imag.fr/Laboratoire/Membres/Wagner_Frederic/rayon-logs.html).

## Use

It is intended to be imported INSTEAD of rayon and you should not call rayon functions directly nor import
its crate.

Logging of iterators is only partially supported.

See [docs.rs](http://docs.rs/rayon_logs) for more info.

## Contact us

We would be **very interested** if your parallel applications have performance issues. Do not hesitate to contact us
for adding functions and or discussion.
