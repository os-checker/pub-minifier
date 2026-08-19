mod a {
    mod b {
        pub fn foo() {}
    }
    use self::b::foo;
}

fn main() {}
