pub mod fixedpoint;
pub mod memory;

pub trait FieldConvert<Out> {
    fn convert(self) -> Out;
}

macro_rules! as_conversion {
    ($From:ty, $To:ty) => {
        impl FieldConvert<$To> for $From {
            fn convert(self) -> $To {
                self as $To
            }
        }
    }
}

impl FieldConvert<bool> for  u8 { fn convert(self) -> bool { self != 0 } }
impl FieldConvert<bool> for u16 { fn convert(self) -> bool { self != 0 } }
impl FieldConvert<bool> for u32 { fn convert(self) -> bool { self != 0 } }
impl FieldConvert<bool> for u64 { fn convert(self) -> bool { self != 0 } }

as_conversion!(bool,  u8);
as_conversion!(bool, u16);
as_conversion!(bool, u32);
as_conversion!(bool, u64);

as_conversion!( u8,  u8);
as_conversion!( u8, u16);
as_conversion!( u8, u32);
as_conversion!( u8, u64);

as_conversion!(u16,  u8);
as_conversion!(u16, u16);
as_conversion!(u16, u32);
as_conversion!(u16, u64);

as_conversion!(u32,  u8);
as_conversion!(u32, u16);
as_conversion!(u32, u32);
as_conversion!(u32, u64);

as_conversion!(u64,  u8);
as_conversion!(u64, u16);
as_conversion!(u64, u32);
as_conversion!(u64, u64);

#[macro_export]
macro_rules! bitfields {
    ($TypeName:ident : $ValueType:ty {
        $(
            $FieldGet:ident, $FieldSet:ident: $FieldType:ty = [$FieldStart:expr, $FieldEnd:expr],
        )*
    }) => {
        #[derive(Copy, Clone)]
        pub struct $TypeName {
            pub value: $ValueType,
        }

        impl $TypeName {
            pub const fn wrap(value: $ValueType) -> $TypeName {
                $TypeName { value }
            }

            $(
                pub fn $FieldGet(&self) -> $FieldType {
                    crate::util::FieldConvert::<$FieldType>::convert((self.value >> $FieldStart) & ((1<<($FieldEnd-$FieldStart+1)) - 1))
                }

                pub fn $FieldSet(&mut self, value: $FieldType) {
                    let value = crate::util::FieldConvert::<$ValueType>::convert(value);
                    self.value = (self.value & !(((1<<($FieldEnd-$FieldStart+1)) - 1) << $FieldStart)) |
                        ((value & ((1<<($FieldEnd-$FieldStart+1)) - 1)) << $FieldStart);
                }
            )*
        }

        impl Default for $TypeName {
            fn default() -> $TypeName {
                Self::wrap(0)
            }
        }
    }
}
