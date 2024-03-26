use core::ops::{Deref, DerefMut};

use macro_support::Deref;

use super::Rect;

#[derive(Debug, Clone, Deref)]
pub struct Square {
    #[deref]
    rect: Rect,
}
