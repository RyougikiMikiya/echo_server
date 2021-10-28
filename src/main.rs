use std::sync::mpsc;
use std::thread;

struct Person {
    name: String,
}
impl Person {
    // fn new (name: String) -> Person {
    //     Person { name: name }
    // }

    fn new<S: Into<String>>(name: S) -> Person {
        Person { name: name.into() }
    }
}

impl From<String> for Person {
    fn from(s: String) -> Self {
        Person { name: s }
    }
}

impl From<&str> for Person {
    fn from(s: &str) -> Self {
        Person { name: s.into() }
    }
}

fn test() {
    Person::new("sdf");
    Person::from("sdfs");
}

fn main() {
    test();
}
