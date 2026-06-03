pub mod single_line_input {
    #[derive(Debug, Clone, Default)]
    pub struct LineInput {
        pub input: String,
        pub cursor_position: usize,
    }

    impl LineInput {
        pub fn new(input: String) -> Self {
            let cursor_position = input.len();
            Self {
                input,
                cursor_position,
            }
        }
    }
}
