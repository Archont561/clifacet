clifacet::__commands!(
    db,
    echo,
    list,
    output = crate::Said,
    request = Request,
    name = "discovered",
    about = "A proof that a folder registers itself"
);

mod _shared;

// A helper: declared for this folder's commands, and never registered as one.
// Re-exported here so a test can reach it.
pub use _shared::greeting;
