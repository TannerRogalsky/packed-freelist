#[cfg(test)]
mod packed_freelist {
    use packed_freelist::{PackedFreelist, AllocationID};

    struct TestStruct {
        pub n: u32,
    }

    #[test]
    fn capacity() {
        let max_objects = 5;
        let p : PackedFreelist<u32> = PackedFreelist::new(max_objects);
        assert_eq!(max_objects, p.capacity());
    }

    #[test]
    fn size() {
        let max_objects = 5;
        let mut p : PackedFreelist<u32> = PackedFreelist::new(max_objects);
        assert_eq!(0, p.size());

        assert!(p.insert(1).is_ok());
        assert_eq!(1, p.size());
    }

    #[test]
    fn contains() {
        {
            let p : PackedFreelist<u32> = PackedFreelist::new(5);
            assert_eq!(p.contains(0), false);
        }

        {
            let mut p : PackedFreelist<u32> = PackedFreelist::new(5);
            let a = p.insert(99).unwrap();
            assert_eq!(p.contains(a), true);
            assert_eq!(p.contains(0), false);
            assert_eq!(p.contains(1), false);
            assert_eq!(p.contains(99), false);
        }
    }

    #[test]
    fn remove() {
        {
            let mut p: PackedFreelist<TestStruct> = PackedFreelist::new(5);
            assert_eq!(p.size(), 0);
            let a = p.insert(TestStruct { n: 0 }).unwrap();
            assert_eq!(p.size(), 1);
            p.remove(a);
            assert_eq!(p.size(), 0);
        }

        {
            let mut p: PackedFreelist<TestStruct> = PackedFreelist::new(3);
            let id1 = p.insert(TestStruct { n: 10 }).unwrap();
            let id2 = p.insert(TestStruct { n: 20 }).unwrap();
            let id3 = p.insert(TestStruct { n: 30 }).unwrap();

            assert_eq!(p.size(), 3);
            assert!(p.contains(id1));
            assert!(p.contains(id2));
            assert!(p.contains(id3));

            p.remove(id1);
            assert_eq!(p.size(), 2);
            assert!(p.contains(id3));
            assert!(p.contains(id2));
            assert!(!p.contains(id1));

            p.remove(id3);
            assert_eq!(p.size(), 1);
            assert!(!p.contains(id3));
            assert!(p.contains(id2));
            assert!(!p.contains(id1));

            p.remove(id2);
            assert_eq!(p.size(), 0);
            assert!(!p.contains(id3));
            assert!(!p.contains(id2));
            assert!(!p.contains(id1));
        }
    }

    #[test]
    fn iterator() {
        {
            let mut p : PackedFreelist<u32> = PackedFreelist::new(5);
            assert_eq!(p.iter().fold(0, |a, &c| a + c), 0);
            assert!(p.insert(1).is_ok());
            assert!(p.insert(2).is_ok());
            assert_eq!(p.iter().fold(0, |a, &c| a + c), 3);
        }

        {
            let mut p : PackedFreelist<TestStruct> = PackedFreelist::new(5);
            assert_eq!(p.iter().fold(0, |a, c| a + c.n), 0);
            assert!(p.insert(TestStruct { n: 1 }).is_ok());
            assert!(p.insert(TestStruct { n: 2 }).is_ok());
            assert_eq!(p.iter().fold(0, |a: u32, c| {
                a + c.n
            }), 3);
        }

        {
            // ensure values are tightly packed
            fn ensure_packdedness<T>(base_ptr: *const T, p: &PackedFreelist<T>) {
                p.iter().enumerate().for_each(|(i, v)| {
                    assert_eq!(v as *const T, unsafe { base_ptr.add(i) });
                });
            }

            const MAX_OBJECTS: usize = 100;
            type TestType = usize;
            let mut p : PackedFreelist<TestType> = PackedFreelist::new(MAX_OBJECTS);
            let ids: Vec<AllocationID> = (0..MAX_OBJECTS).map(|value| p.insert(value as TestType).unwrap()).collect();

            let base_ptr: *const TestType = p.iter().next().unwrap();
            ensure_packdedness(base_ptr, &p);
            ids.iter().for_each(|&id| {
                p.remove(id);
                ensure_packdedness(base_ptr, &p);
            });
        }
    }

    #[test]
    fn index() {
        {
            let mut p : PackedFreelist<u32> = PackedFreelist::new(5);
            let a = p.insert(1).unwrap();
            let b = p.insert(2).unwrap();
            assert_eq!(p[a], 1);
            assert_eq!(p[b], 2);
            assert!(std::panic::catch_unwind(|| p[3]).is_err());
        }
    }
}
