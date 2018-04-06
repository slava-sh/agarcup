macro_rules! impl_getter {
    ($get_name_mut:ident() -> $name:ident: &mut $type:ty) => {
        pub fn $get_name_mut(&mut self) -> &mut $type {
            &mut self.$name
        }
    };
    ($get_name:ident() -> $name:ident: &$type:ty) => {
        pub fn $get_name(&self) -> &$type {
            &self.$name
        }
    };
    ($get_name:ident() -> $name:ident: $type:ty) => {
        pub fn $get_name(&self) -> $type {
            self.$name
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
