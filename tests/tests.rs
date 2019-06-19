mod packed_freelist {
    extern crate rand;

    use packed_freelist::{PackedFreelist, AllocationID};
    use std::error::Error;
    use self::rand::seq::SliceRandom;

    struct TestStruct {
        pub n: u32,
    }

    #[test]
    fn capacity() {
        const CAPACITY: usize = 5;
        let p : PackedFreelist<u32> = PackedFreelist::with_capacity(CAPACITY);
        assert_eq!(CAPACITY, p.capacity());
    }

    #[test]
    fn len() {
        let mut p : PackedFreelist<u32> = PackedFreelist::with_capacity(5);
        assert_eq!(0, p.len());

        assert!(p.insert(1).is_ok());
        assert_eq!(1, p.len());
    }

    #[test]
    fn contains() {
        {
            let p : PackedFreelist<u32> = PackedFreelist::with_capacity(5);
            assert_eq!(p.contains(0), false);
        }

        {
            let mut p : PackedFreelist<u32> = PackedFreelist::with_capacity(5);
            let a = p.insert(99).unwrap();
            assert_eq!(p.contains(a), true);
            assert_eq!(p.contains(0), false);
            assert_eq!(p.contains(1), false);
            assert_eq!(p.contains(99), false);
        }
    }

    #[test]
    fn insert() {
        {
            const CAPACITY: usize = 5;
            let mut p : PackedFreelist<usize> = PackedFreelist::with_capacity(CAPACITY);
            for i in 0..CAPACITY {
                assert!(p.insert(i).is_ok());
            }
            let v = CAPACITY + 1;
            let r = p.insert(v);
            let err = r.unwrap_err();
            assert!(err.source().is_none());
            assert_eq!(format!("{}", err), format!("Failed to acquire allocation with index {}", v));
        }

        {
            let mut p : PackedFreelist<usize> = PackedFreelist::with_capacity(1);
            let id1 = p.insert(1).unwrap();
            p.remove(id1);
            let id2 = p.insert(1).unwrap();
            assert_ne!(id1, id2);
        }
    }

    #[test]
    fn remove() {
        {
            let mut p: PackedFreelist<TestStruct> = PackedFreelist::with_capacity(5);
            assert_eq!(p.len(), 0);
            let a = p.insert(TestStruct { n: 0 }).unwrap();
            assert_eq!(p.len(), 1);
            p.remove(a);
            assert_eq!(p.len(), 0);
        }

        {
//            let order: Vec<u32> = vec![1, 3, 2];
//            let mut ORDER_ITER: std::slice::Iter<u32> = order.iter();
//            impl Drop for TestStruct {
//                fn drop(&mut self) {
//                    unsafe {
//                        assert_eq!(Some(&self.n), ORDER_ITER.next())
//                    }
//                }
//            }

            let mut p: PackedFreelist<TestStruct> = PackedFreelist::with_capacity(3);
            let id1 = p.insert(TestStruct { n: 1 }).unwrap();
            let id2 = p.insert(TestStruct { n: 2 }).unwrap();
            let id3 = p.insert(TestStruct { n: 3 }).unwrap();

            assert_eq!(p.len(), 3);
            assert!(p.contains(id1));
            assert!(p.contains(id2));
            assert!(p.contains(id3));

            p.remove(id1);
            assert_eq!(p.len(), 2);
            assert!(p.contains(id3));
            assert!(p.contains(id2));
            assert!(!p.contains(id1));

            p.remove(id3);
            assert_eq!(p.len(), 1);
            assert!(!p.contains(id3));
            assert!(p.contains(id2));
            assert!(!p.contains(id1));

            p.remove(id2);
            assert_eq!(p.len(), 0);
            assert!(!p.contains(id3));
            assert!(!p.contains(id2));
            assert!(!p.contains(id1));
        }
    }

    #[test]
    fn iterator() {
        {
            let mut p : PackedFreelist<u32> = PackedFreelist::with_capacity(5);
            assert_eq!(p.iter().fold(0, |a, &c| a + c), 0);
            assert!(p.insert(1).is_ok());
            assert!(p.insert(2).is_ok());
            assert_eq!(p.iter().fold(0, |a, &c| a + c), 3);
        }

        {
            let mut p : PackedFreelist<TestStruct> = PackedFreelist::with_capacity(5);
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
            let mut p : PackedFreelist<TestType> = PackedFreelist::with_capacity(MAX_OBJECTS);
            let ids: Vec<AllocationID> = (0..MAX_OBJECTS).map(|value| p.insert(value as TestType).unwrap()).collect();

            let base_ptr: *const TestType = p.iter().next().unwrap();
            ensure_packdedness(base_ptr, &p);

            {
                let rng = &mut rand::thread_rng();
                ids.choose_multiple(rng, ids.len()).for_each(|&id| {
                    p.remove(id);
                    ensure_packdedness(base_ptr, &p);
                });
            }
        }
    }

    #[test]
    fn index() {
        {
            let mut p : PackedFreelist<u32> = PackedFreelist::with_capacity(5);
            let a = p.insert(1).unwrap();
            let b = p.insert(2).unwrap();
            assert_eq!(p[a], 1);
            assert_eq!(p[b], 2);
            assert!(std::panic::catch_unwind(|| p[3]).is_err());
        }
    }
}
