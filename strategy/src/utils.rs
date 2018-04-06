macro_rules! impl_getter {
    ($name:ident() -> $type:ty) => {
        pub fn $name(&self) -> $type {
            self.$name
        }
    };
    ($name:ident() -> &$type:ty) => {
        pub fn $name(&self) -> &$type {
            &self.$name
        }
    };
    ($name_mut:ident() -> $name:ident: &mut $type:ty) => {
        pub fn $name_mut(&mut self) -> &mut $type {
            &mut self.$name
        }
    };
}

macro_rules! impl_setter {
    ($set_name:ident($name:ident: $type:ty)) => {
        pub fn $set_name(&mut self, $name: $type) {
            self.$name = $name;
        }
    };
}
