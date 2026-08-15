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

    pub fn make_b() -> B {
        B
    }
}

mod nested {
    pub mod inner {
        use crate::defs::mid::deep::C;

        pub fn make_c() -> C {
            C
        }
    }
}

fn main() {
    let _ = A;
    let _ = local::make_b();
    let _ = nested::inner::make_c();
}
