//! we duplicate rayon's code here.
//! this is the only possibility to trace rayon's own parallel algorithm
//! without adding tracing hooks inside rayon.
pub mod slice;
