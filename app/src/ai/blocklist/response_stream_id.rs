#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResponseStreamId(String);

impl ResponseStreamId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    #[cfg(test)]
    pub fn new_for_test() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}
