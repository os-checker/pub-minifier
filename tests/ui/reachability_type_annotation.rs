mod ty_defs {
    pub struct S;
    pub enum E {
        A,
    }

    pub mod nested {
        pub struct N;
    }
}

fn takes_s(_: ty_defs::S) {}

fn main() {
    let _s: ty_defs::S = ty_defs::S;
    let _e: ty_defs::E = ty_defs::E::A;
    let _n: ty_defs::nested::N = ty_defs::nested::N;

    takes_s(ty_defs::S);
}
