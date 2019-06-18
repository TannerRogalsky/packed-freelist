use std::vec::Vec;
use std::ops::{Index, Deref};

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

/// Reference replacement to guarantee memory-stability
pub type AllocationID = u32;

#[derive(Debug, Clone)]
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

/// Used to extract the allocation index from an object ID.
const ALLOC_INDEX_MASK: AllocationID = std::u16::MAX as AllocationID;

/// Used to mark an allocation as owning no object. This system's sentinel value.
const TOMBSTONE: u16 = std::u16::MAX;

/// A data structure that provides constant time insertions and deletions and that elements are
/// contiguous in memory.
#[derive(Debug, Clone)]
pub struct PackedFreelist<T> {
    /// Storage for objects
    /// Objects are contiguous, and always packed to the start of the storage.
    /// Objects can be relocated in this storage thanks to the separate list of allocations.
    objects: Vec<T>,

    /// The index in the allocations array for the next allocation to allocate after this one.
    object_alloc_ids: Vec<AllocationID>,

    /// FIFO queue to allocate objects with least ID reuse possible
    allocations: Vec<Allocation>,

    /// When an allocation is freed, the enqueue index struct's next will point to it.
    /// This ensures that allocations are reused as infrequently as possible which reduces the
    /// likelihood that two objects have the same ID. Note objects are still not guaranteed to have
    /// globally unique IDs, since IDs will be reused after N * 2^16 allocations.
    last_allocation: u16,

    /// The next index struct to use for an allocation.
    next_allocation: u16,
}

impl<T> PackedFreelist<T> {
    /// The maximum size allowed by this implementation of a PackedFreelist.
    pub const MAX_SIZE: usize = (TOMBSTONE - 1) as usize;

    /// Constructs a new, empty `PackedFreelist<T>` with the specified capacity.
    ///
    /// The freelist will be able to hold exactly `capacity` elements without reallocating.
    pub fn with_capacity(capacity: usize) -> PackedFreelist<T> {
        assert!(capacity <= Self::MAX_SIZE, "PackedFreelist is too large. Max size is {}.", Self::MAX_SIZE);

        let mut r = PackedFreelist {
            objects: Vec::with_capacity(capacity),
            object_alloc_ids: vec![0; capacity],
            allocations: (0..capacity as u16).map(|i| Allocation {
                allocation_id: i as AllocationID,
                object_index: TOMBSTONE,
                next_allocation: i + 1
            }).collect(),
            last_allocation: (capacity - 1) as u16,
            next_allocation: 0
        };

        if capacity > 0 {
            r.allocations[capacity - 1].next_allocation = 0;
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

    /// Insert an object
    pub fn insert(&mut self, value: T) -> Result<AllocationID, AllocationError> {
        let allocation = self.insert_alloc();

        match allocation {
            Ok(allocation) => {
                let object_index = allocation.object_index as usize;
                let allocation_id = allocation.allocation_id;
                match self.objects.get_mut(object_index) {
                    None => self.objects.push(value),
                    Some(e) => *e = value,
                }
                Ok(allocation_id)
            },
            Err(err) => { Err(err) },
        }
    }

    /// Remove an object
    pub fn remove(&mut self, id: AllocationID) {
        match self.allocations.get((id & ALLOC_INDEX_MASK) as usize) {
            None => { panic!("oh god") },
            Some(allocation) => {
                let last_index = (allocation.allocation_id & ALLOC_INDEX_MASK) as u16;
                match self.objects.get(allocation.object_index as usize) {
                    None => { panic!("no no no no")},
                    Some(_object) => {
                        let last = self.objects.len() - 1;
                        if allocation.object_index as usize != last {
                            self.objects.swap(last, allocation.object_index as usize);
                            self.object_alloc_ids[allocation.object_index as usize] = self.object_alloc_ids[last];
                            let alloc_index = (self.object_alloc_ids[allocation.object_index as usize] & ALLOC_INDEX_MASK) as usize;
                            self.allocations[alloc_index].object_index = allocation.object_index;
                        }
                    },
                }

                self.objects.pop();

                self.allocations[self.last_allocation as usize].next_allocation = last_index;
                self.last_allocation = last_index;
            },
        }

        self.allocations.get_mut((id & ALLOC_INDEX_MASK) as usize).and_then(|a| {
            a.object_index = TOMBSTONE;
            Some(a)
        });
    }

    /// Get number of elements
    pub fn size(&self) -> usize {
        self.objects.len()
    }

    /// Get maximum number of elements
    pub fn capacity(&self) -> usize {
        self.objects.capacity()
    }

    /// Internal allocation logic
    fn insert_alloc(&mut self) -> Result<&Allocation, AllocationError> {
        let len = self.len();
        if len >= self.capacity() {
            return Err(AllocationError { allocation_index: (len + 1) as u16 });
        }

        let allocation = self.allocations.get_mut(self.next_allocation as usize);

        match allocation {
            None => { Err(AllocationError { allocation_index: self.next_allocation }) }
            Some(allocation) => {
                self.next_allocation = allocation.next_allocation;
                allocation.allocation_id += 0x10000;
                allocation.object_index = len as u16;
                self.object_alloc_ids[len] = allocation.allocation_id;

                Ok(allocation)
            },
        }
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

