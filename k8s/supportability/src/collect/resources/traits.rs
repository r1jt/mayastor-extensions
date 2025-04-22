use crate::collect::{constants::DATA_PLANE_CONTAINER_NAME, resources::error::ResourceError};
use async_trait::async_trait;
use downcast_rs::{impl_downcast, Downcast};
use lazy_static::lazy_static;
use std::{collections::HashMap, fmt::Debug};

lazy_static! {
    /// Represents map of resource name to service where resources are hosted
    pub(crate) static ref RESOURCE_TO_CONTAINER_NAME: HashMap<&'static str, &'static str> =
        HashMap::from([
            ("node", DATA_PLANE_CONTAINER_NAME),
            ("pool", DATA_PLANE_CONTAINER_NAME),
            ("nexus", DATA_PLANE_CONTAINER_NAME),
            ("replica", DATA_PLANE_CONTAINER_NAME),
            ("device", DATA_PLANE_CONTAINER_NAME),
        ]);
}

/// Implements functionality to inspect topology information
pub(crate) trait Topologer: Downcast + Debug {
    #[allow(unused)]
    fn get_printable_topology(&self) -> Result<(String, String), ResourceError>;
    fn dump_topology_info(&self, dir_path: String) -> Result<(), ResourceError>;
}
impl_downcast!(Topologer);

/// Resourcer adds functionality to read inputs and build topology information
#[async_trait(?Send)]
pub(crate) trait Resourcer {
    type ID;
    async fn get_topologer(
        &self,
        _id: Option<Self::ID>,
    ) -> Result<Box<dyn Topologer>, ResourceError> {
        panic!("get_topologer is UnImplemented");
    }
}
