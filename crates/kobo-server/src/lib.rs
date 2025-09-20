//! # Starter Rust Project
//!
//! This is a starter Rust project that contains the `.github/workflows` and various
//! configuration files that I find useful for my Rust projects.

/// Print "Hello, world!" to the console.
///
/// # Examples
///
/// ```
/// use starter::hello;
///
/// hello();
/// ```
pub fn hello() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {
        hello();
    }
}
