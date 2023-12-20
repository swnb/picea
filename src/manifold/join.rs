pub struct JoinManifold {
    id: u32,
    object_a_id: u32,
    object_b_id: u32,
}

impl JoinManifold {
    pub fn new(id: u32, object_a_id: u32, object_b_id: u32) -> Self {
        Self {
            id,
            object_a_id,
            object_b_id,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn object_a_id(&self) -> u32 {
        self.object_a_id
    }

    pub fn object_b_id(&self) -> u32 {
        self.object_b_id
    }
}
