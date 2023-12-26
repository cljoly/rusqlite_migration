//! Might eventually hold the async code

/// Add
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 5);
        assert_eq!(result, 7);
    }
}
