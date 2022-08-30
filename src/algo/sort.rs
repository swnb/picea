use std::cmp::Ordering;

pub(crate) trait SortableCollection {
    type Item;

    fn init(&mut self) {}

    fn len(&self) -> usize;

    fn get(&self, index: usize) -> &Self::Item;

    fn swap(&mut self, i: usize, j: usize);

    fn quick_sort<F>(&mut self, compare: F)
    where
        F: Copy + Fn(&Self::Item, &Self::Item) -> Ordering,
    {
        self._quick_sort(0, self.len() - 1, compare)
    }

    #[doc(hidden)]
    fn _quick_sort<F>(&mut self, start_index: usize, end_index: usize, compare: F)
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
            self._quick_sort(start_index, k - 1, compare);
        }

        self._quick_sort(k + 1, end_index, compare);
    }

    fn select_sort_by(&mut self, compare: impl Fn(&Self::Item, &Self::Item) -> Ordering) {
        self.init();

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
