mod defs {
    pub struct A;

    pub mod mid {
        pub struct B;

        pub mod deep {
            pub struct C;
        }
    }
}

use defs::A;

mod local {
    use crate::defs::mid::B;
}

mod nested {
    pub mod inner {
        use crate::defs::mid::deep::C;
    }
}

fn main() {}
