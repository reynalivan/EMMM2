pub(crate) mod adapters {
    pub mod sqlite {
        pub struct Repo;
    }
}
pub mod api {
    pub mod adapters {
        pub use super::super::adapters::*;
    }
}
