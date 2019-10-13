/// Faking const generics :(
macro_rules! impl_circular_buffer {
    ($CircularBuffer:ident, $N:expr) => {
        pub struct $CircularBuffer<T: Default> {
            length: usize,
            tail:   usize,
            buffer: [T; $N],
        }

        impl<T: Default> $CircularBuffer<T> {
            pub fn new() -> $CircularBuffer<T> {
                use std::mem::{MaybeUninit};
                use std::ptr;

                let buffer = {
                    let mut uninit_buffer: [T; $N] = unsafe {
                        MaybeUninit::uninit().assume_init()
                    };

                    // initialize elements
                    for elem in &mut uninit_buffer[0..] {
                        unsafe {
                            ptr::write(elem, T::default());
                        }
                    }

                    uninit_buffer

                    // This doesn't work because Rust is dumb, so we just do it the bad way.
                    // // create an array of uninitialized memory
                    // let mut uninit_buffer: [MaybeUninit<T>; $N] = unsafe {
                    //     MaybeUninit::uninit().assume_init()
                    // };

                    // // initialize elements
                    // for elem in &mut uninit_buffer[0..] {
                    //     unsafe {
                    //         ptr::write(elem.as_mut_ptr(), T::default());
                    //     }
                    // }

                    // // transmute into initialized array
                    // unsafe {
                    //     mem::transmute::<_, [T; $N]>(uninit_buffer)
                    // }
                };

                $CircularBuffer {
                    buffer: buffer,
                    tail:   0,
                    length: 0,
                }
            }

            #[inline]
            pub fn len(&self) -> usize {
                self.length
            }

            #[inline]
            pub fn capacity(&self) -> usize {
                $N
            }

            #[inline]
            pub fn available(&mut self) -> usize {
                $N - self.length
            }

            /// Push to the end and overwrite the start of the buffer we if go out of bounds.
            pub fn push_back_overwrite(&mut self, item: T) {
                self.buffer[self.tail] = item;
                self.tail = (self.tail + 1) % $N;
                self.length = std::cmp::min(self.length + 1, $N);
            }

            pub fn push_back(&mut self, item: T) {
                assert!(self.length < $N, "attempted to push into a full circular buffer");
                self.buffer[self.tail] = item;
                self.tail = (self.tail + 1) % $N;
                self.length += 1;
            }

            pub fn pop_front(&mut self) -> Option<T> {
                if self.length == 0 {
                    None
                } else {
                    let head_idx = self.tail.wrapping_sub(self.length) % $N;
                    let head = std::mem::replace(&mut self.buffer[head_idx], T::default());
                    self.length -= 1;
                    Some(head)
                }
            }

            pub fn peek_front(&self) -> Option<&T> {
                if self.len() == 0 {
                    None
                } else {
                    let head_idx = self.tail.wrapping_sub(self.length) % $N;
                    Some(&self.buffer[head_idx])
                }
            }

            pub fn insert(&mut self, index: usize, element: T) {
                assert!(self.length < $N, "attempted to insert into a full circular buffer");
                assert!(index <= self.length, "attempted to insert after the end of a circular buffer");

                // inserting at the end is the same as a push_back
                if index ==  self.length {
                    self.push_back(element);
                    return;
                }

                let dest_index = self.tail.wrapping_sub(self.length).wrapping_add(index) % self.buffer.len();
                let mut shift_into = self.tail; // start shifting elements into tail

                while shift_into != dest_index {
                    let shift_from = shift_into.wrapping_sub(1) % $N;

                    let shift_into_ptr: *mut T = &mut self.buffer[shift_into] as _;
                    let shift_from_ptr: *mut T = &mut self.buffer[shift_from] as _;
                    unsafe {
                        std::ptr::swap(shift_into_ptr, shift_from_ptr);
                    }

                    shift_into = shift_from;
                }

                self.buffer[dest_index] = element;
                self.tail = (self.tail + 1) % $N;
                self.length += 1;
            }

            pub fn get_internal_buffer(&self) -> &[T] {
                &self.buffer
            }

            pub fn get_internal_head(&self) -> usize {
                self.tail.wrapping_sub(self.length) % $N
            }

            pub fn get(&self, idx: usize) -> Option<&T> {
                if idx < self.length {
                    let idx = self.tail.wrapping_sub(self.length).wrapping_add(idx) % self.buffer.len();
                    Some(&self.buffer[idx])
                } else {
                    None
                }
            }

            pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
                if idx < self.length {
                    let idx = self.tail.wrapping_sub(self.length).wrapping_add(idx) % self.buffer.len();
                    Some(&mut self.buffer[idx])
                } else {
                    None
                }
            }

            pub fn iter<'a>(&'a self) -> CircularBufferIter<'a, T> {
                CircularBufferIter::new(&self.buffer, self.tail, self.length)
            }

            pub fn iter_mut<'a>(&'a mut self) -> CircularBufferIterMut<'a, T> {
                CircularBufferIterMut::new(&mut self.buffer, self.tail, self.length)
            }
        }

        impl<T: Default> Clone for $CircularBuffer<T> where T: Clone {
            fn clone(&self) -> $CircularBuffer<T> {
                let mut new_buffer = $CircularBuffer::new();
                for elem in self.iter() {
                    new_buffer.push_back(elem.clone());
                }
                new_buffer
            }
        }

        impl<T: Default> std::fmt::Debug for $CircularBuffer<T> where T: std::fmt::Debug {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_list().entries(self.iter()).finish()
            }
        }

        impl<T: Default> std::cmp::PartialEq for $CircularBuffer<T> where T: std::cmp::PartialEq {
            fn eq(&self, other: &Self) -> bool {
                self.iter().eq(other.iter())
            }
        }

        impl<T: Default> std::cmp::Eq for $CircularBuffer<T> where T: std::cmp::Eq { /* NOTHING TO DO HERE */ }
    };
}

pub struct CircularBufferIter<'a, T> {
    buffer: &'a [T],
    tail:   usize,
    length: usize,
    offset: usize,
}

impl<'a, T> CircularBufferIter<'a, T> {
    pub fn new(buffer: &'a [T], tail: usize, length: usize) -> CircularBufferIter<'a, T> {
        CircularBufferIter { buffer, tail, length, offset: 0 }
    }
}

impl<'a, T> Iterator for CircularBufferIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        if self.offset >= self.length {
            return None;
        } else {
            let idx = self.tail.wrapping_sub(self.length).wrapping_add(self.offset) % self.buffer.len();
            self.offset += 1;
            Some(&self.buffer[idx])
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.length - self.offset;
        (len, Some(len))
    }
}

impl<'a, T> ExactSizeIterator for CircularBufferIter<'a, T> {
    fn len(&self) -> usize {
        self.length - self.offset
    }
}

pub struct CircularBufferIterMut<'a, T> {
    buf_start:  *mut T,
    buf_len:    usize,

    tail:   usize,
    length: usize,
    offset: usize,

    _marker: std::marker::PhantomData<&'a mut T>
}

impl<'a, T> CircularBufferIterMut<'a, T> {
    pub fn new(buffer: &'a mut [T], tail: usize, length: usize) -> CircularBufferIterMut<'a, T> {
        CircularBufferIterMut {
            tail:   tail,
            length: length,
            offset: 0,

            buf_start:  buffer.as_mut_ptr(),
            buf_len:    buffer.len(),
            _marker:    std::marker::PhantomData,
        }
    }
}

impl<'a, T> Iterator for CircularBufferIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<&'a mut T> {
        if self.offset >= self.length {
            return None;
        } else {
            // Rust is dumb, so I have to do this unsafe stuff to return a mutable reference here:
            let idx = self.tail.wrapping_sub(self.length).wrapping_add(self.offset) % self.buf_len;
            self.offset += 1;
            unsafe {
                self.buf_start.offset(idx as isize).as_mut()
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.length - self.offset;
        (len, Some(len))
    }
}

impl<'a, T> ExactSizeIterator for CircularBufferIterMut<'a, T> {
    fn len(&self) -> usize {
        self.length - self.offset
    }
}

impl_circular_buffer!(CircularBuffer16, 16);
impl_circular_buffer!(CircularBuffer32, 32);
impl_circular_buffer!(CircularBuffer64, 64);

#[cfg(test)]
mod test {
    use super::CircularBuffer16;

    #[test]
    fn test_circular_buffer() {
        let mut buf: CircularBuffer16<u32> = CircularBuffer16::new();

        for i in 0..16 {
            buf.push_back(i);
        }
        assert_eq!(true, (0..16).eq(buf.iter().map(|elem| *elem)), "buf iterator contents not equal");
        assert_eq!(0, buf.available());

        assert_eq!(Some(0), buf.pop_front());
        assert_eq!(Some(1), buf.pop_front());

        buf.iter_mut().for_each(|e| *e = *e * 2);
        assert_eq!(true, (2..16).map(|e| e * 2).eq(buf.iter().map(|elem| *elem)), "buf iterator contents not equal");

        for _ in 0..16 {
            buf.pop_front();
        }

        // force the buffer to wrap around
        for i in 0..16 {
            buf.push_back(i);
        }
        // test iter (and push)
        assert_eq!(true, (0..16).eq(buf.iter().map(|elem| *elem)), "buf iterator contents not equal");
        assert_eq!(0, buf.available());

        // test pop
        assert_eq!(Some(0), buf.pop_front());
        assert_eq!(Some(1), buf.pop_front());

        // test clone and equality
        let mut buf2 = buf.clone();
        assert_eq!(true, buf.iter().eq(buf2.iter()));
        assert_eq!(buf, buf2);

        // test iter_mut
        buf.iter_mut().for_each(|e| *e = *e * 2);
        assert_ne!(buf, buf2);
        assert_eq!(true, (2..16).map(|e| e * 2).eq(buf.iter().map(|elem| *elem)), "buf iterator contents not equal");

        // test insert
        buf2.insert(0, 45);
        buf2.insert(5, 33);
        let expect = [45, 2, 3, 4, 5, 33, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let bufvec = buf2.iter().map(|e| *e).collect::<Vec<_>>();
        assert_eq!(&expect[0..], &bufvec[0..]);

    }

    #[test]
    #[should_panic]
    fn test_circular_buffer_overflow() {
        let mut buf: CircularBuffer16<u32> = CircularBuffer16::new();
        for i in 0..17 {
            buf.push_back(i);
        }
    }

    #[test]
    fn test_circular_buffer_empty_pop() {
        let mut buf: CircularBuffer16<u32> = CircularBuffer16::new();
        assert_eq!(None, buf.pop_front());

        buf.push_back(6);
        assert_eq!(Some(6), buf.pop_front());
        assert_eq!(None, buf.pop_front());
    }
}
