use paste::paste;

macro_rules! id_struct_from {
    ($name:ident) => {
        paste! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            pub struct [<$name Id>]{
                pub id: usize,
            }
        }
    };
}

id_struct_from!(Crate);
id_struct_from!(Module);
id_struct_from!(Symbol);
