use std::cmp::Ordering;

pub(crate) trait SortableCollection {
    type Item;

    fn init(&mut self) {}

    fn len(&self) -> usize;

    fn get(&self, index: usize) -> &Self::Item;

    fn swap(&mut self, i: usize, j: usize);

    fn quick_sort(&mut self) {}

    fn select_sort_by(&mut self, sort_by: impl Fn(&Self::Item, &Self::Item) -> Ordering) {
        self.init();

        for i in 1..self.len() {
            for j in (0..i).rev() {
                if let Ordering::Greater = sort_by(self.get(i), self.get(j)) {
                    self.swap(i, j);
                } else {
                    break;
                }
            }
        }
    }
}
