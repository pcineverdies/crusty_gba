/// Trait BitOperation<T>
///
/// Defines a set of bit-wise operations which are useful while dealing with
/// unsigned numeric types. By using the macro `impl_from_BitOperation`, the
/// trait is implemented over u8, u16, u32, u64 and u128.
pub trait BitOperation<T> {
    fn set_bit(&self, index: T) -> T;
    fn clear_bit(&self, index: T) -> T;
    fn is_bit_set(&self, index: T) -> bool;
    fn is_bit_clear(&self, index: T) -> bool;
    fn flip_bit(&self, index: T) -> T;
    fn get_range(&self, end: T, begin: T) -> T;
}

macro_rules! impl_from_BitOperation {
    ($($uint_type: ty),*) => {
        $(
            impl BitOperation<$uint_type> for $uint_type {

                /// BitOperation::set_bit
                /// @param index [$uint_type]: which bit to set in self, counting from 0
                /// @return [$uint_type]: value with bit set
                fn set_bit(&self, index: $uint_type) -> $uint_type {
                    *self | (1 << index)
                }

                /// BitOperation::flip_bit
                /// @param index [$uint_type]: which bit to flip in self, counting from 0
                /// @return [$uint_type]: value with bit flipped
                fn flip_bit(&self, index: $uint_type) -> $uint_type {
                    *self ^ (1 << index)
                }

                /// BitOperation::clear_bit
                /// @param index [$uint_type]: which bit to clear in self, counting from 0
                /// @return [$uint_type]: value with bit cleared
                fn clear_bit(&self, index: $uint_type) -> $uint_type {
                    *self & (!(1 << index))
                }

                /// BitOperation::is_bit_set
                /// @param index [$uint_type]: bit to check
                /// @return [bool]: true if bit `index` is set, false otherwise
                fn is_bit_set(&self, index: $uint_type) -> bool {
                    (*self & (1 << index)) != 0
                }

                /// BitOperation::is_bit_clear
                /// @param index [$uint_type]: bit to check
                /// @return [bool]: true if bit `index` is clear, false otherwise
                fn is_bit_clear(&self, index: $uint_type) -> bool {
                    !self.is_bit_set(index)
                }

                /// BitOperation::get_range
                ///
                /// Extract range [end, begin] from self.
                ///
                /// @param end [$uint_type]: included ending value of the range
                /// @param begin [$uint_type]: included begin value of the range
                /// @return [u32]: extracted number using the above range
                fn get_range(&self, end: $uint_type, begin: $uint_type) -> $uint_type {
                    if(end < begin){
                        panic!("In `get_range` end ({}) is < than begin ({})", end, begin);
                    }
                    (*self >> begin) & ((1 << (end - begin + 1)) - 1)
                }
            }
        )*
    }
}

impl_from_BitOperation!(u8, u16, u32, u64, u128);

#[cfg(test)]
mod test_bit_operation {

    use crate::common::BitOperation;

    #[test]
    fn test_modify_bit() {
        assert_eq!(0b01011_u32, 0b01001_u32.set_bit(1));
        assert_eq!(0b01011_u8, 0b01001_u8.set_bit(1));
        assert_eq!(0b00001_u32, 0b01001_u32.clear_bit(3));
        assert_eq!(0b00001_u32, 0b01001_u32.flip_bit(3));
        assert_eq!(0b00000_u16, 0b00001_u16.flip_bit(0));
    }

    #[test]
    fn test_check_bits() {
        assert_eq!(true, 0b01001_u32.is_bit_set(3));
        assert_eq!(false, 0b01001_u32.is_bit_set(2));
        assert_eq!(false, 0b01001_u32.is_bit_clear(3));
        assert_eq!(true, 0b01001_u32.is_bit_clear(2));
        assert_eq!(0xaa, 0x08ae21aa_u32.get_range(7, 0));
        assert_eq!(0xe, 0x08ae21aa_u32.get_range(19, 16));
    }
}
