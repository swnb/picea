use std::{
    collections::BTreeSet,
    ops::{BitAnd, BitXor},
};

use crate::math::FloatNum;

#[derive(Debug, Clone)]
pub struct BoundingRect<N = FloatNum> {
    pub top: N,
    pub left: N,
    pub width: N,
    pub height: N,
}

impl BitXor for &BoundingRect {
    type Output = bool;

    // if self and rhs are intersected, return false
    fn bitxor(self, rhs: Self) -> Self::Output {
        let top = self.top;
        let left = self.left;
        let right = left + self.width;
        let bottom = top + self.height;

        let other_top = rhs.top;
        let other_left = rhs.left;
        let other_right = other_left + rhs.width;
        let other_bottom = other_top + rhs.height;

        top > other_bottom || left > other_right || right < other_left || bottom < other_top
    }
}

pub trait BoundingRectProvider {
    fn get_bounding_rect(&self) -> BoundingRect;
}

#[derive(Debug, Clone)]
pub struct QuadTree<T>
where
    T: Ord + BoundingRectProvider + Clone,
{
    root: QuadTreeNode<T, FloatNum>,
}

impl<T> QuadTree<T>
where
    T: Ord + BoundingRectProvider + Clone,
{
    pub fn new(&self, bounding_rect: BoundingRect) -> Self {
        Self {
            root: bounding_rect.into(),
        }
    }

    pub fn insert_element(&mut self, element: T) {
        self.root.insert(element);
    }

    pub fn dfs(&self, callback: impl Fn(&QuadTreeNode<T>) -> bool) {
        self.root.dfs(&callback);
    }

    pub fn search_intersect_element(&self, bounding_rect: &BoundingRect, callback: impl Fn(&T)) {
        self.root.dfs(&|node| {
            if node.is_leaf() {
                node.elements.as_ref().unwrap().iter().for_each(|element| {
                    if !(&element.get_bounding_rect() ^ bounding_rect) {
                        callback(element);
                    }
                });
            }
            node.is_intersect(bounding_rect)
        });
    }

    pub fn dfs_bounding_rect(&self, callback: impl Fn(&BoundingRect)) {
        self.root.dfs(&|node| {
            if node.is_leaf {
                callback(&self.root.bounding_rect);
            }
            true
        });
    }
}

#[derive(Debug, Clone)]
pub struct QuadTreeNode<T, N = FloatNum>
where
    T: Ord + BoundingRectProvider + Clone,
{
    children: Option<Box<[QuadTreeNode<T, N>; 4]>>,
    elements: Option<BTreeSet<T>>,
    bounding_rect: BoundingRect<N>,
    is_leaf: bool,
}

impl<T> From<BoundingRect> for QuadTreeNode<T>
where
    T: Ord + BoundingRectProvider + Clone,
{
    fn from(value: BoundingRect) -> Self {
        Self::new(value)
    }
}

const MAX_ELEMENT_COUNT: usize = 5;

impl<T> QuadTreeNode<T>
where
    T: Ord + BoundingRectProvider + Clone,
{
    pub fn new(bounding_rect: BoundingRect) -> Self {
        Self {
            children: None,
            elements: Some(BTreeSet::new()),
            bounding_rect,
            is_leaf: true,
        }
    }

    pub fn insert(&mut self, element: T) {
        if self.is_leaf() {
            if self.len() > MAX_ELEMENT_COUNT {
                self.split();
                self.insert(element)
            } else {
                self.elements
                    .as_mut()
                    .map(|elements| elements.insert(element));
            }
        } else {
            let bounding_rect = element.get_bounding_rect();

            self.children
                .as_mut()
                .unwrap()
                .iter_mut()
                .filter(|child| child.is_intersect(&bounding_rect))
                .for_each(|child| {
                    child.insert(element.clone());
                });
        }
    }

    pub fn is_intersect(&self, bounding_rect: &BoundingRect) -> bool {
        !(&self.bounding_rect ^ bounding_rect)
    }

    pub fn len(&self) -> usize {
        self.elements
            .as_ref()
            .map(|elements| elements.len())
            .unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_leaf(&self) -> bool {
        self.is_leaf
    }

    fn split(&mut self) {
        let child_width = self.bounding_rect.width / 2.;
        let child_height = self.bounding_rect.height / 2.;

        let mut children: [QuadTreeNode<T>; 4] = [
            BoundingRect {
                top: self.bounding_rect.top,
                left: self.bounding_rect.left,
                width: child_width,
                height: child_height,
            }
            .into(),
            BoundingRect {
                top: self.bounding_rect.top,
                left: self.bounding_rect.left + child_width,
                width: child_width,
                height: child_height,
            }
            .into(),
            BoundingRect {
                top: self.bounding_rect.top + child_height,
                left: self.bounding_rect.left,
                width: child_width,
                height: child_height,
            }
            .into(),
            BoundingRect {
                top: self.bounding_rect.top + child_height,
                left: self.bounding_rect.left + child_width,
                width: child_width,
                height: child_height,
            }
            .into(),
        ];

        if let Some(elements) = self.elements.take() {
            elements.into_iter().for_each(|element| {
                let bounding_rect = &element.get_bounding_rect();
                children
                    .iter_mut()
                    .filter(|child| child.is_intersect(bounding_rect))
                    .for_each(|child| {
                        child.insert(element.clone());
                    });
            })
        }

        self.children = Box::new(children).into();
        self.is_leaf = false;
    }

    pub(crate) fn dfs(&self, callback: &impl Fn(&QuadTreeNode<T>) -> bool) {
        if !callback(self) {
            return;
        }
        if let Some(children) = self.children.as_ref() {
            children.iter().for_each(|child| child.dfs(callback));
        }
    }
}
