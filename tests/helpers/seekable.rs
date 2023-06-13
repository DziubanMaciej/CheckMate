use std::fmt::Display;

pub trait Seekable<T> {
    fn seek(&mut self, arg: T) -> &mut Self;
}

impl<T> Seekable<<T as Iterator>::Item> for T
where
    T: Iterator,
    <T as Iterator>::Item: Display + PartialEq,
{
    fn seek(&mut self, arg: <T as Iterator>::Item) -> &mut Self {
        loop {
            let element = match self.next() {
                Some(x) => x,
                None => panic!("Could not find element \"{arg}\""),
            };
            if element == arg {
                break;
            }
        }
        self
    }
}
