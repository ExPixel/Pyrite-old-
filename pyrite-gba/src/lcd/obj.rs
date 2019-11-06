use crate::hardware::OAM;

pub struct ObjectPriority {
    pub priority_pos:   [(/* offset */ usize, /* length */ usize); 6],

    /// Priority is stored in the lower 3 bits, and obj_index is stored in the higher 8 bits.
    /// So we can compare objects by just comparing the two u16s. Disabled objects are given
    /// priority 5 and therefore will always end up at the end of the array unused.
    pub sorted_objects: [u16;  128],
}

impl ObjectPriority {
    pub fn sorted(oam: &OAM) -> ObjectPriority {
        let mut op = ObjectPriority::new();
        op.sort_objects(oam);
        return op;
    }

    pub fn new() -> ObjectPriority {
        let mut op = ObjectPriority {
            priority_pos:   [(0, 0); 6],
            sorted_objects: [0u16; 128],
        };
        for idx in 0usize..128 { op.sorted_objects[idx] = (idx as u16) << 8; }
        return op;
    }

    #[inline(never)]
    pub fn sort_objects(&mut self, oam: &OAM) {
        self.priority_pos = [(0, 0); 6]; // reset

        for obj_index in 0..128 {
            // let attr_index = obj_index * 8;
            // let attr0_hi = oam[attr_index + 1];
            // let attr2_hi = oam[attr_index + 5];
            // let priority_arr = ((attr2_hi >> 3) & 0x3) as u32 | (4 << 8) | (5 << 16) | (5 << 24);
            // let disable = (attr0_hi & 0x1 != 1) & ((attr0_hi >> 1) & 0x1 == 1);
            // let obj_window = if (attr0_hi >> 3) & 0x3 == 2 { 1 } else { 0 };
            // let priority_sel = ((disable as u32) << 1) | (obj_window as u32);
            // let priority = (priority_arr >> (priority_sel << 3)) as u8; 

            // /// priority cannot be greater than 5 here:
            // unsafe {
            //     (*self.priority_pos.get_unchecked_mut(priority as usize)).1 += 1;
            // }
            // self.sorted_objects[obj_index] |= priority as u16;

            let attr_index = obj_index * 8;
            let attr0_hi = oam[attr_index + 1];

            if attr0_hi & 0x1 != 1 && (attr0_hi >> 1) & 0x1 == 1 {
                self.priority_pos[5].1 += 1;
                self.sorted_objects[obj_index] |= 5;
                continue;
            }

            if (attr0_hi >> 3) & 0x3 == 2 {
                self.priority_pos[4].1 += 1;
                self.sorted_objects[obj_index] |= 4;
                continue;
            }

            let attr2_hi = oam[attr_index + 5];
            let priority = (attr2_hi >> 3) & 0x3;
            self.priority_pos[priority as usize].1 += 1;
            self.sorted_objects[obj_index] |= priority as u16;
        }

        self.sorted_objects.sort_unstable();

        self.priority_pos[1].0 = self.priority_pos[0].1;
        self.priority_pos[2].0 = self.priority_pos[1].0 + self.priority_pos[1].1;
        self.priority_pos[3].0 = self.priority_pos[2].0 + self.priority_pos[2].1;
        self.priority_pos[4].0 = self.priority_pos[3].0 + self.priority_pos[3].1;
        self.priority_pos[5].0 = self.priority_pos[4].0 + self.priority_pos[4].1;
    }

    /// Returns the number of objects with a given priority. Priority 4 is mapped to OBJ window
    /// objects, and priority 5 is mapped to disabled objects.
    pub fn objects_with_priority_count(&self, priority: usize) -> usize {
        return self.priority_pos[priority].1;
    }

    /// Returns all objects with a given priority in the order that they are to be drawn.
    /// Priority 4 is mapped to OBJ window objects, and priority 5 is mapped to disabled objects.
    /// #NOTE object indices are stored in the higher 8 bits of each index in the slice.
    pub fn objects_with_priority(&self, priority: usize) -> &[u16] {
        let start = self.priority_pos[priority].0;
        let end = start + self.priority_pos[priority].1;
        return &self.sorted_objects[start..end];
    }
}
