use std::vec::Vec;
use std::ops::{Index, Deref};
use std::slice::SliceIndex;

/// Reference replacement to guarantee memory-stability
pub type AllocationID = u32;

/// Used to extract the allocation index from an object ID.
const ALLOC_INDEX_MASK: AllocationID = std::u16::MAX as AllocationID;

/// Used to mark an allocation as owning no object. This system's sentinel value.
const TOMBSTONE: u16 = std::u16::MAX;

/// Indicates an error when attempting to allocate an object
#[derive(Debug, Clone)]
pub struct AllocationError {
    allocation_index: u16,
}

impl std::error::Error for AllocationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

impl std::fmt::Display for AllocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Failed to acquire allocation with index {}", self.allocation_index)
    }
}

#[derive(Clone)]
struct Allocation {
    /// The ID of this allocation:
    ///  - The 16 LSBs store the index of this allocation in the list of allocations
    ///  - The 16 MSBs store the number of times this allocation struct was used to allocate an object
    ///     - This is used as a (non-perfect) counter-measure to reusing IDs for objects.
    allocation_id: AllocationID,

    /// The index in the objects array which stores the allocated object for this allocation.
    object_index: u16,

    /// The index in the allocations array for the next allocation to allocate after this one.
    next_allocation: u16,
}

#[derive(Clone)]
pub struct PackedFreelist<T> {
    /// Storage for objects
    /// Objects are contiguous, and always packed to the start of the storage.
    /// Objects can be relocated in this storage thanks to the separate list of allocations.
    objects: Vec<T>,
    num_objects: usize,

    /// The index in the allocations array for the next allocation to allocate after this one.
    object_alloc_ids: Vec<AllocationID>,

    /// FIFO queue to allocate objects with least ID reuse possible
    allocations: Vec<Allocation>,

    /// When an allocation is freed, the enqueue index struct's next will point to it.
    /// This ensures that allocations are reused as infrequently as possible
    /// which reduces the likelihood that two objects have the same ID.
    /// Note objects are still not guaranteed to have globally unique IDs, since IDs will be reused after N * 2^16 allocations.
    last_allocation: u16,

    /// The next index struct to use for an allocation.
    next_allocation: u16,
}

impl<T: Default + Clone> PackedFreelist<T> {
    pub fn new(max_objects: usize) -> PackedFreelist<T> {
        assert!(max_objects < TOMBSTONE as usize, "PackedFreelist is too large. Max size is {}.", TOMBSTONE - 1);

        let mut r = PackedFreelist {
            objects: vec![T::default(); max_objects],
            num_objects: 0,
            object_alloc_ids: vec![0; max_objects],
            allocations: (0..max_objects as u16).map(|i| Allocation {
                allocation_id: i as AllocationID,
                object_index: TOMBSTONE,
                next_allocation: i + 1
            }).collect(),
            last_allocation: (max_objects - 1) as u16,
            next_allocation: 0
        };

        if max_objects > 0 {
            r.allocations[max_objects - 1].next_allocation = 0;
        }

        r
    }

    /// Query for an ID.
    /// Returns true if the ID corresponds to an object in the list. False otherwise.
    pub fn contains(&self, id: AllocationID) -> bool {
        let allocation = self.allocations.get((id & ALLOC_INDEX_MASK) as usize);

        match allocation {
            None => { false },
            Some(allocation) => { allocation.allocation_id == id && allocation.object_index != TOMBSTONE },
        }
    }

    /// Copy an object into
    pub fn insert(&mut self, val: T) -> Result<AllocationID, AllocationError> {
        let allocation = self.insert_alloc();

        match allocation {
            Ok(allocation) => {
                let object_index = allocation.object_index as usize;
                let allocation_id = allocation.allocation_id;
                let dest = &mut self.objects[object_index];
                std::mem::replace(dest, val);
                Ok(allocation_id)
            },
            Err(err) => { Err(err) },
        }
    }

    pub fn remove(&mut self, id: AllocationID) {
        match self.allocations.get((id & ALLOC_INDEX_MASK) as usize) {
            None => { panic!("oh god") },
            Some(allocation) => {
                let last_index = (allocation.allocation_id & ALLOC_INDEX_MASK) as u16;

                match self.objects.get(allocation.object_index as usize) {
                    None => { panic!("no no no no")},
                    Some(object) => {
                        let last = self.num_objects - 1;
                        if allocation.object_index as usize != last {
                            self.objects.swap(last, allocation.object_index as usize);
                            self.object_alloc_ids[allocation.object_index as usize] = self.object_alloc_ids[last];
                            let alloc_index = (self.object_alloc_ids[allocation.object_index as usize] & ALLOC_INDEX_MASK) as usize;
                            self.allocations[alloc_index].object_index = allocation.object_index;
                        }
                    },
                }

                self.objects.pop();
                self.num_objects -= 1;

                self.allocations[self.last_allocation as usize].next_allocation = last_index;
                self.last_allocation = last_index;
            },
        }

        self.allocations.get_mut((id & ALLOC_INDEX_MASK) as usize).and_then(|a| {
            a.object_index = TOMBSTONE;
            Some(a)
        });
    }

    pub fn size(&self) -> usize {
        self.num_objects
    }

    pub fn capacity(&self) -> usize {
        self.objects.capacity()
    }

    fn insert_alloc(&mut self) -> Result<&Allocation, AllocationError> {
        if self.num_objects >= self.capacity() {
            return Err(AllocationError { allocation_index: self.next_allocation });
        }

        let allocation = self.allocations.get_mut(self.next_allocation as usize);

        match allocation {
            None => { Err(AllocationError { allocation_index: self.next_allocation }) }
            Some(allocation) => {
                self.next_allocation = allocation.next_allocation;
                allocation.allocation_id += 0x10000;
                allocation.object_index = self.num_objects as u16;
                self.object_alloc_ids[self.num_objects] = allocation.allocation_id;

                self.num_objects += 1;

                Ok(allocation)
            },
        }
    }
}

//pub struct PackedFreelistIter<'a, T> {
//    list: &'a PackedFreelist<'a, T>,
//    current: usize,
//}
//
//impl<'a, T> Iterator for PackedFreelistIter<'a, T> {
//    type Item = &'a T;
//
//    fn next(&mut self) -> Option<Self::Item> {
//        let v = self.list.objects.get(self.current);
//        self.current += 1;
//        v
//    }
//}
//
//impl<'a, T> IntoIterator for PackedFreelist<'a, T> {
//    type Item = T;
//    type IntoIter = PackedFreelistIter<'a, Self::Item>;
//
//    fn into_iter(self) -> Self::IntoIter {
//        PackedFreelistIter {
//            list: self.objects,
//            current: 0,
//        }
//    }
//}

impl<T> IntoIterator for PackedFreelist<T> {
    type Item = T;
    type IntoIter = ::std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.objects.into_iter()
    }
}

impl<T> Index<AllocationID> for PackedFreelist<T> {
    type Output = T;

    fn index(&self, index: AllocationID) -> &Self::Output {
        let alloc = self.allocations.get((index & ALLOC_INDEX_MASK) as usize).unwrap();
        self.objects.get(alloc.object_index as usize).unwrap()
    }
}

impl<T> Deref for PackedFreelist<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.objects.deref()
    }
}

