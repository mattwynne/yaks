// Display port trait - abstraction for displaying results to user

pub trait DisplayPort {
    /// Display success message
    #[allow(dead_code)]
    fn success(&self, message: &str);

    /// Display error message
    #[allow(dead_code)]
    fn error(&self, message: &str);

    /// Display informational message
    fn info(&self, message: &str);
}
