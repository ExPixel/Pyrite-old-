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
        macro_rules! mkobj {
            ($Index:expr, $Priority:expr) => {
                (($Index as u16) << 8) | ($Priority as u16)
            }
        }

        let mut priority_pos = [(0, 0); 6];
        let mut objects = [0u16; 128];

        let mut enabled_index   = 0; // start inserting enabled objects here
        let mut disabled_index  = 128; // start inserting disabled objects here

        for obj_index in 0..128 {
            let attr_index = obj_index * 8;
            let attr0_hi = oam[attr_index + 1];

            if attr0_hi & 0x1 != 1 && (attr0_hi >> 1) & 0x1 == 1 {
                priority_pos[5].1 += 1;
                disabled_index -= 1;
                objects[disabled_index] = mkobj!(obj_index, 5);
                continue;
            }

            if (attr0_hi >> 3) & 0x3 == 2 {
                priority_pos[4].1 += 1;
                objects[enabled_index] |= mkobj!(obj_index, 4);
                enabled_index += 1;
                continue;
            }

            let attr2_hi = oam[attr_index + 5];
            let priority = (attr2_hi >> 3) & 0x3;
            priority_pos[priority as usize].1 += 1;
            objects[enabled_index] = mkobj!(obj_index, priority);
            enabled_index += 1;
        }

        // this we only bother sorting enabled objects:
        (&mut objects[0..(disabled_index)]).sort_unstable();

        priority_pos[1].0 = priority_pos[0].1;
        priority_pos[2].0 = priority_pos[1].0 + priority_pos[1].1;
        priority_pos[3].0 = priority_pos[2].0 + priority_pos[2].1;
        priority_pos[4].0 = priority_pos[3].0 + priority_pos[3].1;
        priority_pos[5].0 = disabled_index;

        return ObjectPriority {
            priority_pos:   priority_pos,
            sorted_objects: objects,
        }
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
