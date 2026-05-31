#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TextLocation {
    Output {
        section_index: usize,
        line_index: usize,
    },
    Query {
        input_index: usize,
    },
    Action {
        action_index: usize,
        line_index: usize,
    },
}
