#[derive(Clone, Debug, Default)]
pub struct Key;

#[derive(Clone, Debug, Default)]
pub struct KeySequence {
    pub keys: Vec<Key>,
}
