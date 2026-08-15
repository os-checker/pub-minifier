mod defs {
    pub struct Item;

    pub mod inner {
        pub struct Deep;
    }
}

pub mod exports {
    pub use crate::defs::Item;
    pub use crate::defs::inner::Deep;
}

fn main() {
    let _ = exports::Item;
    let _ = exports::Deep;
}
