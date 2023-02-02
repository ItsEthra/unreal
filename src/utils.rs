use std::slice::from_raw_parts;

pub(crate) fn terminated<'a, T: PartialEq>(mut ptr: *const T, last: &T) -> &'a [T] {
    if ptr.is_null() {
        return &[];
    }

    unsafe {
        let start = ptr;
        while last != &*ptr {
            ptr = ptr.add(1)
        }

        from_raw_parts(start, ptr.offset_from(start) as usize)
    }
}

#[test]
fn test_terminated() {
    let s = "Hello!\0";
    assert_eq!(terminated(s.as_ptr(), &0), b"Hello!");
}
