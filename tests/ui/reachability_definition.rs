mod level1 {
    pub struct Point {
        pub x: i32,
        pub y: i32,
    }

    pub enum Shape {
        Dot { p: Point },
        Pair(Point, Point),
    }

    pub union Bits {
        pub i: i32,
        pub u: u32,
    }

    pub fn use_fields() {
        let p = Point { x: 1, y: 2 };
        let _ = p.x + p.y;

        let s = Shape::Dot {
            p: Point { x: 3, y: 4 },
        };
        let _ = match s {
            Shape::Dot { p } => p.x,
            Shape::Pair(a, b) => a.x + b.y,
        };

        let bits = Bits { i: 7 };
        let _ = unsafe { bits.i };
    }

    pub mod level2 {
        pub struct Holder {
            pub p: super::Point,
        }

        pub fn make_holder() -> Holder {
            Holder {
                p: super::Point { x: 10, y: 20 },
            }
        }
    }
}

fn main() {
    level1::use_fields();
    let _ = level1::level2::make_holder();
}
