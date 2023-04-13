pub trait PushRet<T> {
    fn push_ret(&mut self, t: T) -> &mut Vec<T>;
}

impl<T> PushRet<T> for Vec<T> {
    fn push_ret(&mut self, t: T) -> &mut Vec<T> {
        self.push(t);
        self
    }
}
