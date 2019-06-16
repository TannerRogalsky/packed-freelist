#[cfg(test)]
mod tests {
    use packed_freelist::PackedFreelist;

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
            let mut p : PackedFreelist<u32> = PackedFreelist::new(5);
            let a = p.insert(99).unwrap();
            assert_eq!(p.size(), 1);
            p.remove(a);
            assert_eq!(p.size(), 0);
        }

        {
            const CAPACITY: usize = 100;
//            let mut constructed_count = 0;

            #[derive(Default, Copy)]
            struct Test;
//            impl std::marker::Copy for Test;
            impl std::clone::Clone for Test {
                fn clone(&self) -> Self {
                    *self
                }
            };

            let mut p: PackedFreelist<Test> = PackedFreelist::new(CAPACITY);
            assert_eq!(p.size(), 0);
            let a = p.insert(Test {}).unwrap();
            p.remove(a);
            assert_eq!(p.size(), 0);
        }
    }

    #[test]
    fn iterator() {
        #[derive(Default, Clone)]
        struct TestStruct {
            pub n: u32,
        }

        {
            let mut p : PackedFreelist<u32> = PackedFreelist::new(5);
            assert_eq!(p.iter().fold(0, |a, &c| a + c), 0);
            p.insert(1);
            p.insert(2);
            assert_eq!(p.iter().fold(0, |a, &c| a + c), 3);
        }

        {
            let mut p : PackedFreelist<TestStruct> = PackedFreelist::new(5);
            assert_eq!(p.iter().fold(0, |a, c| a + c.n), 0);
            p.insert(TestStruct { n: 1 });
            p.insert(TestStruct { n: 2 });
            assert_eq!(p.iter().fold(0, |a: u32, c| {
                a + c.n
            }), 3);
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
        }
    }
}
