use std::mem::MaybeUninit;
use std::ptr;

/// Rust arrays suck (until const generics hopefully) so I have to do this to initialize a large array.
#[inline]
pub fn new_array<ItemType: Sized + Clone, ArrayType: Sized>(default_value: ItemType) -> ArrayType {
    assert!(
        std::mem::size_of::<ArrayType>() % std::mem::size_of::<ItemType>() == 0,
        "sizeof(ArrayType) must be a multiple of sizeof(ItemType)"
    );

    unsafe {
        let mut arr_uninit = MaybeUninit::<ArrayType>::uninit();
        let arr_ptr = arr_uninit.as_mut_ptr().cast::<ItemType>();
        let count = std::mem::size_of::<ArrayType>() / std::mem::size_of::<ItemType>();

        for idx in 0..count {
            ptr::write(arr_ptr.add(idx), default_value.clone());
        }

        arr_uninit.assume_init()
    }
}
