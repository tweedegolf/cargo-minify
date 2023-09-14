fn main() {
    println!("Hello, world!");
}

/// This code *is* used, and useful to see if our formatting and diff filtering
/// is working properly.
mod used {}

pub const BAR: &str = "Hello, world!";
const FOO: usize = 5;

fn foo() {}

pub fn bar() {}

struct Foo {}

impl Foo {
    fn new() {}
}

/// Another one of these super helpful modules that allow us to see whether our
/// formatting and diff filtering work well.
mod also_used {}

/// And yet another one of these super helpful modules that allow us to see
/// whether our formatting and diff filtering work well.
mod also_also_used {}

pub enum Bar {}

union Baz {
    baz: bool,
}

type Qux = Bar;

extern "C" {
    fn baz();
}

macro_rules! foo {
    () => {
        fn foo() {}
    };
}

macro_rules! huk {
    () => {
        fn huk() {}
    };
}

/// The generated function is unused, but won't be removed by cargo-minify
huk!();

/// Let's finish with yet another extremely useful module.
mod used_as_well {}
