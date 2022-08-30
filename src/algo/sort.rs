use std::cmp::Ordering;

trait PrivateSortableCollection: SortableCollection {
    fn quick_sort<F>(&mut self, start_index: usize, end_index: usize, compare: F)
    where
        F: Copy + Fn(&Self::Item, &Self::Item) -> Ordering,
    {
        if start_index >= end_index {
            return;
        }

        let partial = self.get(start_index) as *const Self::Item;

        let mut k = start_index;

        for index in (start_index + 1)..=end_index {
            if !compare(self.get(index), unsafe { &*partial }).is_gt() {
                k += 1;
                self.swap(index, k);
            }
        }

        self.swap(start_index, k);

        if k != 0 {
            PrivateSortableCollection::quick_sort(self, start_index, k - 1, compare);
        }

        PrivateSortableCollection::quick_sort(self, k + 1, end_index, compare);
    }
}

impl<T: ?Sized> PrivateSortableCollection for T where T: SortableCollection {}

pub(crate) trait SortableCollection {
    type Item;

    fn len(&self) -> usize;

    fn get(&self, index: usize) -> &Self::Item;

    fn swap(&mut self, i: usize, j: usize);

    fn quick_sort<F>(&mut self, compare: F)
    where
        F: Copy + Fn(&Self::Item, &Self::Item) -> Ordering,
    {
        PrivateSortableCollection::quick_sort(self, 0, self.len() - 1, compare);
    }

    fn select_sort_by(&mut self, compare: impl Fn(&Self::Item, &Self::Item) -> Ordering) {
        for i in 1..self.len() {
            let mut index = i;
            for j in (0..index).rev() {
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
