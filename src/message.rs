pub fn get_message() -> &'static str {
    "Hello, world!"
}

#[cfg(test)]
mod tests {
    use super::get_message;

    #[test]
    fn returns_hello_world_message() {
        assert_eq!(get_message(), "Hello, world!");
    }
}
