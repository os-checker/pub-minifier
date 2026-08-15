mod ctor {
    pub struct Unit;
    pub struct Tuple(pub i32);
    pub struct Named {
        pub v: i32,
    }

    pub enum E {
        U,
        T(i32),
        N { v: i32 },
    }
}

fn main() {
    let _ = ctor::Unit;
    let _ = ctor::Tuple(1);
    let _ = ctor::Named { v: 2 };

    let _ = ctor::E::U;
    let _ = ctor::E::T(3);
    let _ = ctor::E::N { v: 4 };
}
