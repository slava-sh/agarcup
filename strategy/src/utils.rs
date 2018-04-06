macro_rules! impl_getter {
    ($name:ident() -> $type:ty) => {
        pub fn $name(&self) -> $type {
            self.$name()
        }
    };
    ($name:ident() -> &$type:ty) => {
        pub fn $name(&self) -> &$type {
            &self.$name()
        }
    };
    ($name:ident() -> &mut $type:ty) => {
        pub fn $name(&mut self) -> &mut $type {
            &mut self.$name()
        }
    };
}
