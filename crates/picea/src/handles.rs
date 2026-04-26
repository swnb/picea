//! Typed opaque handles for the stable v1 world API.

use serde::{Deserialize, Serialize};

macro_rules! define_handle {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        pub struct $name(u64);

        impl $name {
            /// Sentinel handle used by `Default` for descriptors before a real world-owned handle
            /// is assigned.
            pub const INVALID: Self = Self(u64::MAX);

            #[allow(dead_code)]
            pub(crate) const fn from_raw_parts(index: u32, generation: u32) -> Self {
                Self(((generation as u64) << 32) | index as u64)
            }

            #[allow(dead_code)]
            pub(crate) const fn index(self) -> Option<usize> {
                if self.0 == u64::MAX {
                    None
                } else {
                    Some((self.0 & u32::MAX as u64) as usize)
                }
            }

            #[allow(dead_code)]
            pub(crate) const fn generation(self) -> Option<u32> {
                if self.0 == u64::MAX {
                    None
                } else {
                    Some((self.0 >> 32) as u32)
                }
            }

            /// Returns `true` when the handle refers to a world-managed slot.
            pub const fn is_valid(self) -> bool {
                self.0 != u64::MAX
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::INVALID
            }
        }
    };
}

define_handle!(
    BodyHandle,
    "Opaque identifier for a body owned by a [`World`](crate::world::World)."
);
define_handle!(
    ColliderHandle,
    "Opaque identifier for a collider owned by a [`World`](crate::world::World)."
);
define_handle!(
    JointHandle,
    "Opaque identifier for a joint owned by a [`World`](crate::world::World)."
);
define_handle!(
    ContactId,
    "Opaque identifier for a contact in debug and event streams."
);
define_handle!(
    ContactFeatureId,
    "Opaque identifier for one contact point feature inside a collider-pair manifold."
);
define_handle!(
    ManifoldId,
    "Opaque identifier for a contact manifold in debug and event streams."
);

/// Monotonic world revision used by read-only caches such as queries and debug outputs.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct WorldRevision(u64);

impl WorldRevision {
    #[allow(dead_code)]
    pub(crate) const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub(crate) const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// Returns the raw monotonically increasing revision number.
    pub const fn get(self) -> u64 {
        self.0
    }
}
