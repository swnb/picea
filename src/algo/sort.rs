use std::cmp::Ordering;

struct SortableCollectionWrapper<'a, T>(&'a mut T)
where
    T: SortableCollection + ?Sized;

// private method
impl<'a, T> SortableCollectionWrapper<'a, T>
where
    T: SortableCollection + ?Sized,
{
    fn get(&self, index: usize) -> &T::Item {
        self.0.get(index)
    }

    fn swap(&mut self, i: usize, j: usize) {
        self.0.swap(i, j)
    }

    fn quick_sort<F>(&mut self, start_index: usize, end_index: usize, compare: &F)
    where
        F: Fn(&T::Item, &T::Item) -> Ordering,
    {
        if start_index >= end_index {
            return;
        }

        if end_index - start_index <= 15 {
            self.insertion_sort(start_index, end_index, compare);
            return;
        }

        let partial = self.get(start_index) as *const T::Item;

        let mut k = start_index;

        for index in (start_index + 1)..=end_index {
            if !compare(self.get(index), unsafe { &*partial }).is_gt() {
                k += 1;
                self.swap(index, k);
            }
        }

        self.swap(start_index, k);

        if k != 0 {
            self.quick_sort(start_index, k - 1, compare);
        }

        self.quick_sort(k + 1, end_index, compare);
    }

    fn insertion_sort<F>(&mut self, start_index: usize, end_index: usize, compare: &F)
    where
        F: Fn(&T::Item, &T::Item) -> Ordering,
    {
        for i in (start_index + 1)..=end_index {
            let mut index = i;
            for j in (start_index..index).rev() {
                if !compare(self.get(index), self.get(j)).is_gt() {
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
pub(crate) trait SortableCollection {
    type Item;

    fn len(&self) -> usize;

    fn get(&self, index: usize) -> &Self::Item;

    fn swap(&mut self, i: usize, j: usize);

    fn quick_sort<F>(&mut self, compare: F)
    where
        F: Fn(&Self::Item, &Self::Item) -> Ordering,
    {
        let length = self.len();
        SortableCollectionWrapper(self).quick_sort(0, length - 1, &compare);
    }

    fn insertion_sort(&mut self, compare: impl Fn(&Self::Item, &Self::Item) -> Ordering) {
        let length = self.len();
        SortableCollectionWrapper(self).insertion_sort(0, length - 1, &compare)
    }
}
