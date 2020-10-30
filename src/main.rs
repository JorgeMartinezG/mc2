fn main() {
    println!("Hello world");
}

#[cfg(test)]
#[path = "./tests.rs"]
mod tests;
