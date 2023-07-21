use std::{
    cmp::Ordering,
    ops::{Bound, Deref, DerefMut, Index, RangeBounds},
};

struct SortableCollectionWrapper<'a, T>(&'a mut T)
where
    T: SortableCollection + ?Sized;

impl<'a, T> Deref for SortableCollectionWrapper<'a, T>
where
    T: SortableCollection + ?Sized,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a, T> DerefMut for SortableCollectionWrapper<'a, T>
where
    T: SortableCollection + ?Sized,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

// private method
impl<'a, T> SortableCollectionWrapper<'a, T>
where
    T: SortableCollection + ?Sized,
{
    fn quick_sort<F>(&mut self, range: impl RangeBounds<usize>, compare: &F)
    where
        F: Fn(&T::Item, &T::Item) -> Ordering,
    {
        let (start_index, end_index) = RangeBoundsToIndex(range).to_index_bound();

        if start_index >= end_index {
            return;
        }

        if end_index - start_index <= 15 {
            self.insertion_sort(start_index..=end_index, compare);
            return;
        }

        let partial = &self[start_index] as *const T::Item;

        let mut k = start_index;

        for index in (start_index + 1)..=end_index {
            if !compare(&self[index], unsafe { &*partial }).is_gt() {
                k += 1;
                self.swap(index, k);
            }
        }

        self.swap(start_index, k);

        if k != 0 {
            self.quick_sort(start_index..k, compare);
        }

        self.quick_sort((k + 1)..=end_index, compare);
    }

    fn insertion_sort<F>(&mut self, range: impl RangeBounds<usize>, compare: &F)
    where
        F: Fn(&T::Item, &T::Item) -> Ordering,
    {
        let (start_index, end_index) = RangeBoundsToIndex(range).to_index_bound();

        for i in (start_index + 1)..end_index {
            let mut index = i;
            for j in (start_index..index).rev() {
                if compare(&self[index], &self[j]).is_lt() {
                    // TODO opt
                    self.swap(index, j);
                    index -= 1;
                } else {
                    break;
                }
            }
        }
    }
}

// any collection impl this trait can be sort
pub(crate) trait SortableCollection: Index<usize, Output = Self::Item> {
    type Item;

    fn len(&self) -> usize;

    fn swap(&mut self, i: usize, j: usize);

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn quick_sort<F>(&mut self, compare: F)
    where
        F: Fn(&Self::Item, &Self::Item) -> Ordering,
    {
        if self.is_empty() {
            return;
        }

        let length = self.len();
        SortableCollectionWrapper(self).quick_sort(0..length, &compare);
    }

    fn insertion_sort(&mut self, compare: impl Fn(&Self::Item, &Self::Item) -> Ordering) {
        if self.is_empty() {
            return;
        }

        let length = self.len();
        SortableCollectionWrapper(self).insertion_sort(0..length, &compare)
    }
}

struct RangeBoundsToIndex<R>(R);

impl<R> RangeBoundsToIndex<R>
where
    R: RangeBounds<usize>,
{
    fn to_index_bound(&self) -> (usize, usize) {
        use Bound::*;
        let inner_value = &self.0;
        let start_index = match inner_value.start_bound() {
            Unbounded => unimplemented!("don't accept unbounded range"),
            Included(&n) => n,
            Excluded(&n) => n + 1,
        };
        let end_index = match inner_value.end_bound() {
            Unbounded => unimplemented!("don't accept unbounded range"),
            Included(&n) => n,
            Excluded(&n) => n - 1,
        };
        (start_index, end_index)
    }
}
