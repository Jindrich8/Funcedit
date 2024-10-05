use std::ops::Range;

use bit_iter::BitIter;

const USIZE_BITS: usize = usize::BITS as usize;

#[derive(Debug, Clone)]
pub struct Iter<'a> {
    elements: &'a Vec<usize>,
    element_index: usize,
    current: BitIter<usize>,
    len: usize,
}

macro_rules! iter {
    ($self:ident,$next:ident) => {{
        let current = &mut $self.current;
        let elements = &$self.elements;
        let element_index = &mut $self.element_index;
        let len = &mut $self.len;
        while *len > 0 {
            if let Some(item) = current.$next() {
                *len -= 1;
                return Some(item + element_index.saturating_sub(1) * USIZE_BITS);
            } else if *element_index < elements.len() {
                let element = elements[*element_index];
                *current = BitIter::from(element.to_owned());
                *element_index += 1;
            } else {
                break;
            }
        }
        None
    }};
}

impl<'a> Iterator for Iter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        iter!(self, next)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        iter!(self, next_back)
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {}

#[derive(Debug, Clone)]
pub struct IntoIter {
    elements: Box<[usize]>,
    element_index: usize,
    current: BitIter<usize>,
    len: usize,
}

impl Iterator for IntoIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        iter!(self, next)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl DoubleEndedIterator for IntoIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        iter!(self, next_back)
    }
}

impl ExactSizeIterator for IntoIter {}

#[derive(Debug, Default)]
pub struct BitSet {
    vec: Vec<usize>,
    len: usize,
}

impl BitSet {
    pub fn new() -> Self {
        Self {
            vec: Vec::new(),
            len: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            vec: Vec::with_capacity(capacity / USIZE_BITS),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    fn index_and_mask(pos: usize) -> (usize, usize) {
        (pos / USIZE_BITS, 1 << (pos % USIZE_BITS))
    }

    fn index_and_from_mask(pos: usize) -> (usize, usize) {
        (pos / USIZE_BITS, usize::MAX << (pos % USIZE_BITS))
    }

    pub fn contains(&self, key: usize) -> bool {
        let (index, mask) = Self::index_and_mask(key);
        if let Some(element) = self.vec.get(index) {
            (element & mask) != 0
        } else {
            false
        }
    }

    fn element_ref(&mut self, index: usize) -> &mut usize {
        if self.vec.len() <= index {
            self.vec.resize(index + 1, 0);
        }

        let element = self.vec.get_mut(index).unwrap();
        element
    }

    fn element_ref_and_mask(&mut self, key: usize) -> (&mut usize, usize) {
        let (index, mask) = Self::index_and_mask(key);
        let element = self.element_ref(index);
        (element, mask)
    }

    fn element_ref_index_and_from_mask(&mut self, key: usize) -> (&mut usize, usize, usize) {
        let (index, mask) = Self::index_and_from_mask(key);
        let element = self.element_ref(index);
        (element, index, mask)
    }

    /// Returns if key was inserted into set
    pub fn insert(&mut self, key: usize) -> bool {
        let (element_ref, mask) = self.element_ref_and_mask(key);
        let old_element = *element_ref;
        let new_element = old_element | mask;

        *element_ref = new_element;
        let inserted = new_element != old_element;
        if inserted {
            self.len += 1;
        }
        self.assert_len();
        inserted
    }

    /// Returns if key was presented in set
    pub fn toggle(&mut self, key: usize) -> bool {
        let (element_ref, mask) = self.element_ref_and_mask(key);
        let old_element = *element_ref;
        let new_element = old_element ^ mask;

        *element_ref = new_element;
        let removed = (old_element & mask) != 0;
        self.len = if removed { self.len - 1 } else { self.len + 1 };
        self.assert_len();
        removed
    }

    fn assert_len(&self) {
        debug_assert_eq!(self.len, self.iter().count());
    }

    /// Returns if key was removed from set
    pub fn remove(&mut self, key: usize) -> bool {
        let (element_ref, mask) = self.element_ref_and_mask(key);
        let old_element = *element_ref;
        let new_element = old_element & (!mask);

        *element_ref = new_element;
        let removed = new_element != old_element;
        if removed {
            self.len -= 1;
        }
        removed
    }

    pub fn retain(&mut self, mut callback: impl FnMut(usize) -> bool) {
        for (element_index, element) in self.vec.iter_mut().enumerate() {
            let mut mask: usize = 1;
            for bit in BitIter::from(*element) {
                let retain = callback(bit + element_index * USIZE_BITS);
                if !retain {
                    *element &= !mask;
                }
                mask <<= 1;
            }
        }
    }

    pub fn insert_range(&mut self, range: Range<usize>) {
        let mut len = self.len;

        let (first_index, first_from_mask) = Self::index_and_from_mask(range.start);
        let (end_index, end_from_mask) = Self::index_and_from_mask(range.end);
        let first_ref = self.element_ref(first_index);
        let first = *first_ref;

        if first_index == end_index {
            let mask = first_from_mask ^ end_from_mask;
            *first_ref = first | mask;
            self.len += (first | (!mask)).count_zeros() as usize;
            self.assert_len();
            return;
        }

        len += (!first_from_mask | first).count_zeros() as usize;
        *first_ref = first | first_from_mask;

        let end_ref = self.element_ref(end_index);
        let end = *end_ref;
        len += (end | end_from_mask).count_zeros() as usize;
        *end_ref |= !end_from_mask;
        let range = first_index.saturating_add(1)..end_index;
        if !range.is_empty() {
            let slice = &mut self.vec[range];
            slice.iter().for_each(|e| len += e.count_zeros() as usize);
            slice.fill(usize::MAX);
        }
        self.len = len;
        self.assert_len();
    }

    pub fn remove_range(&mut self, range: Range<usize>) {
        let mut len = self.len;

        let (first_index, first_from_mask) = Self::index_and_from_mask(range.start);
        let (end_index, end_from_mask) = Self::index_and_from_mask(range.end);
        let first_ref = self.element_ref(first_index);
        let first = *first_ref;
        if first_index == end_index {
            let mask = first_from_mask ^ end_from_mask;
            *first_ref = first & (!mask);
            self.len -= (first & mask).count_ones() as usize;
            self.assert_len();
            return;
        }

        len -= (first & first_from_mask).count_ones() as usize;
        *first_ref &= !first_from_mask;

        let end_ref = self.element_ref(range.end);
        let end = *end_ref;
        len -= (end & (!end_from_mask)).count_ones() as usize;
        *end_ref &= end_from_mask;
        let range = first_index.saturating_add(1)..end_index;
        if !range.is_empty() {
            let slice = &mut self.vec[range];
            slice.iter().for_each(|e| len -= e.count_ones() as usize);
            slice.fill(0);
        }
        self.len = len;
        self.assert_len();
    }

    /// Returns iterator visiting keys in ascending order
    pub fn iter<'a>(&'a self) -> Iter {
        Iter {
            elements: &self.vec,
            element_index: 0,
            current: BitIter::from(0),
            len: self.len,
        }
    }
}

impl IntoIterator for BitSet {
    type Item = usize;

    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            elements: self.vec.into_boxed_slice(),
            element_index: 0,
            current: BitIter::from(0),
            len: self.len,
        }
    }
}

impl<Iter: Iterator<Item = usize>> From<Iter> for BitSet {
    fn from(value: Iter) -> Self {
        let mut set = Self::with_capacity(value.size_hint().0);
        for i in value {
            set.insert(i);
        }
        set
    }
}
