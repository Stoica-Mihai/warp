pub mod render;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SubshellSource {
    Command(String),
}
