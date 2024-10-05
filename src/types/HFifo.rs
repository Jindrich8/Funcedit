use std::{alloc, collections::VecDeque, io::Write, mem::MaybeUninit, ptr::NonNull};

#[derive(Debug, Copy, Clone, PartialEq)]
enum RecAccessOptions {
    First,
    Last,
}

struct ValueRec<'a, T> {
    value: &'a T,
    alloc_size: usize,
}

#[derive(Debug, Clone, Copy)]
enum Value<T> {
    WithPadding(T),
    NoPadding(T),
}

impl<T> Value<T> {
    pub fn new(value: T, padding: usize) -> Self {
        if padding > 0 {
            Self::WithPadding(value)
        } else {
            Self::NoPadding(value)
        }
    }
}

pub struct HFifo {
    data: Vec<u8>,
}

impl HFifo {
    pub fn alloc<T>(&mut self, value: T)
    where
        T: Sized + 'static,
    {
        let size = std::mem::size_of::<Value<T>>();
        if size < 1 {
            return;
        }
        let index = self.data.len();
        self.reserve_for_alloc::<T>();
        {
            let curr_len = self.data.len();
            let curr_capacity = self.data.capacity();
            let slice = self.data.as_mut_slice();
            let ptr = std::ptr::from_mut(&mut slice[index]);
            let offset = ptr.align_offset(std::mem::align_of::<Value<T>>());
            let alloc_size = offset + size;
            assert!((alloc_size + curr_len) < curr_capacity);
            Self::write_padding(&mut slice[index..index + offset]);
            let value = Value::new(value, offset);
            unsafe {
                std::ptr::write(ptr.add(offset) as *mut Value<T>, value);
            }
            self.data.truncate(index + alloc_size);
        }
    }

    pub fn get_index<T>(&self) -> Option<usize>
    where
        T: Sized + 'static,
    {
        if let Some(_) = self.get_value::<T>() {
            Some(self.data.len() - std::mem::size_of::<Value<T>>())
        } else {
            None
        }
    }

    fn get_value_rec<'a, T>(data: &'a [u8], options: RecAccessOptions) -> Option<ValueRec<'a, T>>
    where
        T: Sized + 'static,
    {
        let size = std::mem::size_of::<Value<T>>();
        let s = data;
        let (value, index) = Self::get_value_and_index_from_data::<T>(&s, options)?;
        let (has_padding, value) = match value {
            Value::NoPadding(v) => (false, v),
            Value::WithPadding(v) => (true, v),
        };

        let offset = if has_padding {
            match options {
                RecAccessOptions::First => index,
                RecAccessOptions::Last => Self::read_padding(&s[..index]),
            }
        } else {
            0
        };
        Some(ValueRec {
            value,
            alloc_size: size + offset,
        })
    }

    pub fn dealloc<T>(&mut self)
    where
        T: Sized + 'static,
    {
        let s = self.data.as_slice();
        if let Some(rec) = Self::get_value_rec::<T>(s, RecAccessOptions::Last) {
            self.data.truncate(self.data.len() - rec.alloc_size);
        }
    }

    fn get_mut_value<T>(&mut self) -> Option<&mut Value<T>>
    where
        T: Sized + 'static,
    {
        let s = self.data.as_mut_slice();
        let len = s.len();
        let size = std::mem::size_of::<Value<T>>();
        if size < 1 || len < size {
            return None;
        }
        let ptr = std::ptr::from_mut(&mut s[len - size]);

        let value_ptr = ptr as *mut Value<T>;
        assert!(value_ptr.is_aligned() && !value_ptr.is_null());
        let value = unsafe { value_ptr.as_mut().unwrap() };
        Some(value)
    }

    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Sized + 'static,
    {
        let val = self.get_mut_value();
        let value = val.and_then(|val| {
            Some(match val {
                Value::NoPadding(v) => v,
                Value::WithPadding(v) => v,
            })
        });
        value
    }

    fn get_value_and_index_from_data<T>(
        data: &[u8],
        options: RecAccessOptions,
    ) -> Option<(&Value<T>, usize)>
    where
        T: Sized + 'static,
    {
        let s = data;
        let len = s.len();
        let size = std::mem::size_of::<Value<T>>();
        if size < 1 || len < size {
            return None;
        }
        let index = match options {
            RecAccessOptions::First => s.as_ptr().align_offset(std::mem::align_of::<Value<T>>()),
            RecAccessOptions::Last => len - size,
        };
        let ptr = std::ptr::from_ref(&s[index]);

        let value_ptr = ptr as *const Value<T>;
        assert!(value_ptr.is_aligned() && !value_ptr.is_null());
        let value = unsafe { value_ptr.as_ref().unwrap() };
        Some((value, index))
    }

    fn get_value<T>(&self) -> Option<&Value<T>>
    where
        T: Sized + 'static,
    {
        let s = self.data.as_slice();
        Self::get_value_and_index_from_data(s, RecAccessOptions::Last).and_then(|(v, _)| Some(v))
    }

    pub fn get<T>(&self) -> Option<&T>
    where
        T: Sized + 'static,
    {
        let val = self.get_value::<T>();
        let value = val.and_then(|val| {
            Some(match val {
                Value::NoPadding(v) => v,
                Value::WithPadding(v) => v,
            })
        });
        value
    }

    fn write_padding(padding: &mut [u8]) {
        let mut len = padding.len();
        if len > 0 {
            let mut p = padding.iter_mut();
            while len > i8::MAX as usize {
                *p.next_back().unwrap() = (-((len as i8) & i8::MAX)) as u8;
                len >>= i8::BITS - 1;
            }
            if len > 0 {
                *p.next_back().unwrap() = (len as u8) & i8::MAX as u8;
            }
        }
    }

    fn read_padding(data: &[u8]) -> usize {
        let mut len: usize = 0;
        for p in data.iter().map(|p| *p as i8) {
            len |= (p & i8::MAX) as usize;
            len <<= i8::BITS - 1;
            if p >= 0 {
                break;
            }
        }
        len
    }

    fn reserve_for_alloc<T>(&mut self)
    where
        T: Sized + 'static,
    {
        let size = std::mem::size_of::<Value<T>>();
        let new_min_len = self.data.len() + size;
        let capacity = self.data.capacity();
        if new_min_len > capacity {
            self.data.resize(size * 2 - 1, 0);
        } else {
            let (len, alloc_size) = {
                let slice = self.data.as_mut_slice();
                let len = slice.len();
                let ptr = slice.as_mut_ptr();
                let end_ptr = unsafe { ptr.add(len) };
                let offset = end_ptr.align_offset(std::mem::align_of::<Value<T>>());
                (len, offset + size)
            };
            let new_len = len + alloc_size;
            if self.data.capacity() < new_len {
                self.data.resize(size * 2 - 1, 0);
            } else {
                self.data.resize(new_len, 0);
            }
        }
    }

    // fn align_padding(&self, layout: &alloc::Layout) -> usize {
    //     let align = layout.align();
    //     let mask = align;

    //     (align - (self.byte_offset + self.data.as_ref() & mask)) & mask
    // }
}

pub struct HFifoPrevIter<'a> {
    data: &'a [u8],
}

impl<'a> HFifoPrevIter<'a> {
    pub fn prev<T>(&mut self) -> Option<&T>
    where
        T: Sized + 'static,
    {
        let rec = HFifo::get_value_rec(&self.data, RecAccessOptions::Last)?;
        self.data = &self.data[..self.data.len() - rec.alloc_size];
        Some(rec.value)
    }
}

pub struct HFifoNextIter<'a> {
    data: &'a [u8],
}

impl<'a> HFifoNextIter<'a> {
    pub fn next<T>(&mut self) -> Option<&T>
    where
        T: Sized + 'static,
    {
        let rec = HFifo::get_value_rec(&self.data, RecAccessOptions::First)?;
        self.data = &self.data[rec.alloc_size..];
        Some(rec.value)
    }
}
