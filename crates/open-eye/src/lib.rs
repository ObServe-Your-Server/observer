pub mod collector;

fn test() {
    println!("test");
}

//test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test() {
        test();
    }
}